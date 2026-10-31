package com.localcamera.app.encoder

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaCodecList
import android.media.MediaFormat
import android.util.Log
import android.view.Surface

/**
 * Escolha e configuração do codec para o [VideoEncoder] (cartão F2-T08):
 * enumeração via `MediaCodecList` (**sem nome fixo**), fallback
 * perfil (High → Main → Baseline, por capability) × `KEY_LOW_LATENCY`
 * (try/ignore — G781B rejeita) × `bitrate-mode` CBR.
 * Separado da classe para manter o [VideoEncoder] dentro do limite de linhas.
 */

/** `MimeTypes.VIDEO_AVC` (classe removida dos stubs a partir da API 34). */
internal const val MIME_AVC = "video/avc"

/** Tag de log comum (classe + configuração). */
internal const val LOG_TAG = "VideoEncoder"

// Chaves/parâmetros ausentes dos stubs públicos (android.jar 34+); valores
// da fonte AOSP (frameworks/av) e receita validada no spike S3 (aparelho):
internal const val KEY_BITRATE_MODE = "bitrate-mode"
internal const val PRIORITY_REAL_TIME = 0 // = CodecCapabilities.PRIORITY_REAL_TIME

/**
 * Enumera os encoders H.264 do aparelho: devolve [candidates] (para
 * [pickEncoder]) e as capacidades completas (para o log de diagnóstico).
 */
internal fun scanEncoders(
    cfg: EncoderConfig,
): Pair<List<EncoderCandidate>, Map<String, MediaCodecInfo.CodecCapabilities>> {
    val infos = MediaCodecList(MediaCodecList.REGULAR_CODECS).codecInfos
    val candidates = ArrayList<EncoderCandidate>()
    val capsMap = HashMap<String, MediaCodecInfo.CodecCapabilities>()
    for (i in infos) {
        if (!i.isEncoder) continue
        // `getCapabilitiesForType` LANÇA (IllegalArgumentException) para codec
        // sem o mime — não devolve null; envelopa e segue.
        val caps = try { i.getCapabilitiesForType(MIME_AVC) } catch (e: IllegalArgumentException) { null } ?: continue
        capsMap[i.name] = caps
        candidates += EncoderCandidate(
            name = i.name,
            canEncode = caps.isFormatSupported(probeFormat(cfg)),
            supportedProfiles = (caps.profileLevels ?: emptyArray()).map { it.profile },
            // Input surface obrigatória (câmera → inputSurface). Não existe
            // constante FEATURE_INPUT_SURFACE na API (conf. AOSP main/8.1):
            // o framework permite `createInputSurface()` só quando o formato
            // de superfície está nos formatos de entrada do codec.
            supportsInputSurface = (caps.colorFormats ?: IntArray(0))
                .contains(MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface),
        )
    }
    return candidates to capsMap
}

/** Formato de sondagem (`isFormatSupported`): resolução/fps/bitrate pedidos. */
internal fun probeFormat(cfg: EncoderConfig): MediaFormat {
    val fmt = MediaFormat.createVideoFormat(MIME_AVC, cfg.width, cfg.height)
    fmt.setInteger(MediaFormat.KEY_BIT_RATE, cfg.bitrateKbps * 1000)
    fmt.setInteger(MediaFormat.KEY_FRAME_RATE, cfg.fps)
    return fmt
}

/** Perfis aceitos pela capability, na ordem preferida; `null` = sem KEY_PROFILE. */
internal fun profileAttempts(supported: List<Int>): List<Int?> =
    H264Profile.preferred.filter { wanted ->
        H264Profile.acceptedFor(wanted).any { supported.contains(it) }
    } + null

/** Tenta (perfil × lowLatency × CBR); devolve (perfil, lowLatency) vencedor. */
internal fun configureCodec(
    c: MediaCodec,
    surface: Surface,
    cfg: EncoderConfig,
    profiles: List<Int?>,
): Pair<Int?, Boolean> {
    for (p in profiles) {
        if (tryConfigure(c, surface, cfg, p, lowLatency = true)) return p to true
        if (tryConfigure(c, surface, cfg, p, lowLatency = false)) return p to false
    }
    // Último recurso: sem a chave "bitrate-mode" (encoder que a rejeite).
    val last = profiles.lastOrNull()
    if (tryConfigure(c, surface, cfg, last, lowLatency = false, cbr = false)) return last to false
    throw IllegalStateException("encoder ${c.name} rejeitou todas as configurações")
}

internal fun tryConfigure(
    c: MediaCodec,
    surface: Surface,
    cfg: EncoderConfig,
    profile: Int?,
    lowLatency: Boolean,
    cbr: Boolean = true,
): Boolean {
    try {
        // Surface de entrada é obrigatória no configure (contrato do MediaCodec).
        c.configure(buildFormat(cfg, profile, lowLatency, cbr), surface, null, 0)
        return true
    } catch (t: IllegalArgumentException) {
        Log.w(
            LOG_TAG,
            "configure rejeitado (profile=${profile?.let(H264Profile::name) ?: "any"}, " +
                "lowLatency=$lowLatency, cbr=$cbr): ${t.message}",
        )
        return false
    }
}

/** Formato do encoder: CBR 8 Mbps padrão do cartão, tempo real, sem B-frames. */
internal fun buildFormat(
    cfg: EncoderConfig,
    profile: Int?,
    lowLatency: Boolean,
    cbr: Boolean,
): MediaFormat {
    val fmt = MediaFormat.createVideoFormat(MIME_AVC, cfg.width, cfg.height)
    fmt.setInteger(MediaFormat.KEY_BIT_RATE, cfg.bitrateKbps * 1000)
    if (cbr) {
        // "bitrate-mode": CBR pelo valor de EncoderCapabilities (receta do S3).
        fmt.setInteger(KEY_BITRATE_MODE, MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR)
    }
    fmt.setInteger(MediaFormat.KEY_FRAME_RATE, cfg.fps)
    fmt.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, cfg.keyframeIntervalS)
    fmt.setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
    fmt.setInteger(MediaFormat.KEY_PRIORITY, PRIORITY_REAL_TIME) // tempo real (cartão)
    // `MediaFormat` só tem setInteger: os flags booleanos usam 1 (convenção AOSP).
    fmt.setInteger(MediaFormat.KEY_PREPEND_HEADER_TO_SYNC_FRAMES, 1)
    if (profile != null) fmt.setInteger(MediaFormat.KEY_PROFILE, profile)
    if (lowLatency) fmt.setInteger(MediaFormat.KEY_LOW_LATENCY, 1)
    return fmt
}

/** Níveis suportados pelo perfil escolhido (diagnóstico; valor bruto, como no S3). */
internal fun levelsFor(caps: MediaCodecInfo.CodecCapabilities?, profile: Int?): String {
    if (profile == null || caps == null) return "n/a"
    val accepted = H264Profile.acceptedFor(profile)
    val levels = caps.profileLevels
        ?.filter { it.profile in accepted }
        ?.map { it.level }
        ?.distinct()
        ?: emptyList()
    return levels.joinToString(",")
}
