package com.localcamera.app.camera

import android.content.Context
import android.graphics.SurfaceTexture
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraManager
import android.media.MediaCodecList
import android.os.Build
import android.util.Range
import android.util.Size
import org.json.JSONArray
import org.json.JSONObject

/**
 * Sonda o aparelho real e monta o JSON `caps` do HELLO, no formato do vetor
 * `hello_pair` de `docs/protocol-vectors.json`:
 *
 * ```json
 * {"cameras":[{"id":"0","facing":"back","modes":[{"w":1920,"h":1080,"fps":[30]}]}],
 *  "concurrent":false,"audio":true,"codecs":["h264"]}
 * ```
 *
 * Regras (cartão F2-T06 + spike S3):
 * - Nada é fixado na mão: ids, facing, resoluções de vídeo e faixas de fps vêm
 *   das `CameraCharacteristics` do aparelho (Camera2);
 * - fps: inclui [TARGET_FPS] quando alguma faixa de AE cobre 30; NUNCA 60
 *   (S3: `offers60=false`);
 * - `concurrent`: capacidade real medida do hardware; o usuário valida o valor
 *   no GATE (passo 5 do cartão);
 * - [FIELD_H264_ENCODERS]: campo extra de diagnóstico (o servidor ignora campos
 *   extras — escolha do encoder é do F2-T08).
 */
object CapabilitiesProbe {

    /** Padrão do cartão (CHEATSHEET: 1080p30). O 60 nunca entra em `modes` (S3). */
    private const val TARGET_FPS = 30

    /**
     * MIME H.264 — valor da constante `MimeTypes.VIDEO_AVC` (a classe
     * `MimeTypes` foi removida dos stubs públicos a partir da API 34).
     */
    private const val MIME_VIDEO_AVC = "video/avc"

    /** Campo extra de diagnóstico: nomes dos encoders H.264 encontrados. */
    const val FIELD_H264_ENCODERS = "h264Encoders"

    /** Sonda o aparelho e devolve o JSON `caps` (string). */
    fun probe(context: Context): String {
        val cameraManager = context.getSystemService(Context.CAMERA_SERVICE) as CameraManager
        val cameras = JSONArray()
        cameraManager.cameraIdList.forEach { id ->
            cameras.put(cameraJson(id, cameraManager.getCameraCharacteristics(id)))
        }

        val encoders = h264EncoderNames()
        val root = JSONObject()
        root.put("cameras", cameras)
        root.put("concurrent", concurrent(cameraManager))
        root.put("audio", true) // capability; o envio de áudio vem na Fase 8
        root.put("codecs", JSONArray().apply { if (encoders.isNotEmpty()) put("h264") })
        root.put(FIELD_H264_ENCODERS, encoders.toJSONArray())
        return root.toString()
    }

    private fun cameraJson(id: String, ch: CameraCharacteristics): JSONObject {
        val fps = supportedFps(ch)
        // Só entram resoluções de vídeo com fps suportado: modo é
        // resolução × fps; sem fps o modo não é utilizável pelo PC.
        val modes = JSONArray()
        videoSizes(ch).forEach { size ->
            if (fps.isNotEmpty()) {
                val mode = JSONObject()
                mode.put("w", size.width)
                mode.put("h", size.height)
                mode.put("fps", fps.toJSONArray())
                modes.put(mode)
            }
        }
        return JSONObject()
            .put("id", id)
            .put("facing", facingName(ch.get(CameraCharacteristics.LENS_FACING)))
            .put("modes", modes)
    }

    /**
     * Resoluções de saída de vídeo (formato SurfaceTexture = entrada de
     * encoder), medidas do aparelho.
     *
     * API 34+ usa a chave `SCALER_STREAM_CONFIGURATION_MAP` (renomeada oficial;
     * as chaves antigas foram removidas do android.jar a partir da 34). Em
     * runtimes mais antigos o campo novo nem existe (`NoSuchFieldError`), por
     * isso a guarda por versão; o [legacyVideoSizes] cobre Android < 34.
     */
    private fun videoSizes(ch: CameraCharacteristics): List<Size> {
        if (Build.VERSION.SDK_INT >= 34) {
            try {
                val map = ch.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP)
                val sizes = map?.getOutputSizes(SurfaceTexture::class.java)
                if (!sizes.isNullOrEmpty()) return sizes.toList().distinct()
            } catch (t: Throwable) {
                // chave sintética indisponível neste runtime → fallback abaixo
            }
        }
        return legacyVideoSizes(ch)
    }

    /**
     * Fallback para Android < 34: a chave
     * `SCALER_AVAILABLE_STREAM_CONFIGURATIONS` foi removida do `android.jar`
     * (compileSdk 34+) mas ainda existe no runtime desses aparelhos —
     * consulta por reflexão (o método público correspondente não compila).
     */
    @Suppress("UNCHECKED_CAST")
    private fun legacyVideoSizes(ch: CameraCharacteristics): List<Size> {
        return try {
            val key = Class
                .forName("android.hardware.camera2.CameraCharacteristics")
                .getField("SCALER_AVAILABLE_STREAM_CONFIGURATIONS")
                .get(null)
            val map = ch.get(key as CameraCharacteristics.Key<Any>) ?: return emptyList()
            val sizes = map.javaClass
                .getMethod("getOutputSizes", Class::class.java)
                .invoke(map, SurfaceTexture::class.java) as? Array<out Any>
            sizes?.mapNotNull { it as? Size }?.distinct() ?: emptyList()
        } catch (t: Throwable) {
            emptyList()
        }
    }

    /**
     * fps do padrão do cartão (30), incluído quando alguma faixa de
     * `CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES` do aparelho cobre 30.
     * 60 fps NUNCA entra (S3: `offers60=false`).
     */
    private fun supportedFps(ch: CameraCharacteristics): List<Int> {
        val ranges: Array<out Range<Int>>? =
            ch.get(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES)
        return if (ranges?.any { it.lower <= TARGET_FPS && TARGET_FPS <= it.upper } == true) {
            listOf(TARGET_FPS)
        } else {
            emptyList()
        }
    }

    /**
     * Capacidade real de câmeras simultâneas (Camera2).
     * - API 31+: `getConcurrentCameraIds()` devolve os grupos de ids que podem
     *   abrir juntos; `true` se existir grupo com 2+ câmeras.
     * - Abaixo da 31: não há API pública de consulta → `false` (medição
     *   honesta; o usuário valida no GATE).
     */
    private fun concurrent(cameraManager: CameraManager): Boolean {
        if (Build.VERSION.SDK_INT < 31) return false
        return try {
            cameraManager.concurrentCameraIds.any { it.size >= 2 }
        } catch (t: Exception) {
            false
        }
    }

    /**
     * Nomes dos encoders H.264 do aparelho (`MediaCodecList`), para
     * diagnóstico (a escolha do encoder é do F2-T08).
     */
    private fun h264EncoderNames(): List<String> = try {
        MediaCodecList(MediaCodecList.REGULAR_CODECS).codecInfos
            .filter { it.isEncoder && it.getCapabilitiesForType(MIME_VIDEO_AVC) != null }
            .map { it.name }
            .sorted()
    } catch (t: Throwable) {
        emptyList()
    }

    private fun facingName(facing: Int?): String = when (facing) {
        CameraCharacteristics.LENS_FACING_FRONT -> "front"
        CameraCharacteristics.LENS_FACING_EXTERNAL -> "external"
        CameraCharacteristics.LENS_FACING_BACK -> "back"
        else -> "unknown" // medição honesta: facing não reconhecido
    }

    private fun <T> List<T>.toJSONArray(): JSONArray =
        JSONArray().apply { forEach { put(it) } }
}
