package com.localcamera.app.camera

import android.os.Build
import android.util.Log
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * IT-2.04 — `CapabilitiesProbe` no aparelho (cartão F2-T06).
 *
 * Valida que o JSON `caps` está no formato do vetor `hello_pair`
 * (`docs/protocol-vectors.json`) e foi medido do aparelho:
 * - ao menos 1 câmera `back` com modo 1920×1080 e 30 fps;
 * - 60 fps nunca aparece em `modes` (S3: `offers60=false`);
 * - `concurrent` coerente: API < 31 → `false` (sem API pública de consulta);
 *   API 31+ → valor medido, impresso para o usuário validar no GATE
 *   (passo 5 do cartão);
 * - encoder H.264 presente (diagnóstico em `h264Encoders`).
 */
@RunWith(AndroidJUnit4::class)
class CapabilitiesProbeTest {

    private val validFacings = setOf("back", "front", "external")

    @Test
    fun probe_caps_no_formato_dos_vetores_com_1080p30_em_back() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext.applicationContext
        val capsJson = CapabilitiesProbe.probe(context)
        // Impresso para o humano revisar (passo 5 do cartão F2-T06).
        Log.i(TAG, "caps = $capsJson")

        val caps = JSONObject(capsJson)

        // Campos de raiz do vetor `hello_pair`.
        assertTrue("faltam cameras", caps.has("cameras"))
        assertTrue("concurrent não é booleano", caps.get("concurrent") is Boolean)
        assertEquals("audio deve ser true (capability)", true, caps.get("audio"))
        assertTrue("faltam codecs", caps.has("codecs"))
        assertTrue(
            "codecs deve conter h264: ${caps.getJSONArray("codecs")}",
            "h264" in caps.getJSONArray("codecs").stringList()
        )

        // Diagnóstico (campo extra do cartão): encoders H.264 do aparelho.
        val encoders = caps.getJSONArray(CapabilitiesProbe.FIELD_H264_ENCODERS)
        assertTrue("nenhum encoder H.264 encontrado: $capsJson", encoders.length() > 0)
        Log.i(TAG, "h264Encoders = $encoders")

        // Estrutura por câmera: id (string), facing válido, modos {w,h,fps}.
        val cameras = caps.getJSONArray("cameras")
        assertTrue("nenhuma câmera listada", cameras.length() > 0)

        var back1080p30 = false
        for (i in 0 until cameras.length()) {
            val camera = cameras.getJSONObject(i)
            assertTrue("falta id na câmera $i", camera.has("id"))
            assertTrue("id deve ser string", camera.get("id") is String)

            val facing = camera.getString("facing")
            assertTrue("facing inválido: $facing", facing in validFacings)

            val modes = camera.getJSONArray("modes")
            for (j in 0 until modes.length()) {
                val mode = modes.getJSONObject(j)
                val width = mode.getInt("w")
                val height = mode.getInt("h")
                assertTrue("modo inválido ${width}x$height", width > 0 && height > 0)

                val fps = mode.getJSONArray("fps").intList()
                assertTrue("modo ${width}x$height sem fps", fps.isNotEmpty())
                fps.forEach {
                    assertTrue(
                        "60 fps é proibido (S3: offers60=false): ${width}x$height",
                        it != 60
                    )
                }
                if (facing == "back" && width == 1920 && height == 1080 && 30 in fps) {
                    back1080p30 = true
                }
            }
        }
        assertTrue("nenhuma câmera back com modo 1080x30: $capsJson", back1080p30)

        // `concurrent` coerente: abaixo da API 31 não há API pública de
        // consulta, então a medição honesta é false. Na 31+ o valor é medido
        // do hardware e validado pelo usuário no GATE.
        val concurrent = caps.getBoolean("concurrent")
        Log.i(TAG, "concurrent = $concurrent (API ${Build.VERSION.SDK_INT}) — validar no GATE")
        if (Build.VERSION.SDK_INT < 31) {
            assertEquals("API < 31 deve reportar concurrent=false", false, concurrent)
        }
    }

    private fun JSONArray.intList(): List<Int> = (0 until length()).map { getInt(it) }

    private fun JSONArray.stringList(): List<String> = (0 until length()).map { getString(it) }

    private companion object {
        private const val TAG = "CapabilitiesProbeTest"
    }
}
