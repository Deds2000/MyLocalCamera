package com.localcamera.app.encoder

/**
 * Configuração do encoder H.264 (cartão F2-T08).
 * Padrão do cartão (CHEATSHEET): 1080p30 CBR 8 Mbps, keyframe a cada 1 s.
 * O modo dual (720p30 ~4 Mbps — Fase 7) reaproveita esta classe com valores
 * próprios; aqui o tamanho do quadro é apenas validado como `> 0`.
 *
 * Kotlin puro: nenhum tipo Android aqui (testável em JVM).
 */
data class EncoderConfig(
    val width: Int = 1920,
    val height: Int = 1080,
    val fps: Int = 30,
    val bitrateKbps: Int = 8000,
    val keyframeIntervalS: Int = 1,
) {
    init {
        require(width > 0 && height > 0) { "resolução ${width}x$height deve ser > 0" }
        require(fps > 0) { "fps $fps deve ser > 0" }
        require(bitrateKbps > 0) { "bitrate $bitrateKbps kbps deve ser > 0" }
        require(keyframeIntervalS > 0) { "intervalo de keyframe $keyframeIntervalS s deve ser > 0" }
    }
}

/**
 * Perfis H.264 (Annex A.5.1) — mesmos valores de
 * `MediaCodecInfo.CodecProfileLevel.AVCProfile*` (confirmados no android.jar
 * 36), redeclarados aqui para manter a regra de escolha pura (JVM).
 */
object H264Profile {
    const val BASELINE = 1
    const val MAIN = 2
    const val HIGH = 8
    const val CONSTRAINED_BASELINE = 7
    const val CONSTRAINED_HIGH = 24

    /** Ordem preferida (protocolo §10): High com fallback Main → Baseline. */
    val preferred: List<Int> = listOf(HIGH, MAIN, BASELINE)

    /**
     * Perfis que um encoder pode declarar para satisfazer [wanted]: o perfil
     * em si ou sua variante "Constrained" (Constrained High ⊂ High,
     * Constrained Baseline ⊂ Baseline; pedir o perfil "cheio" quando só a
     * variante existe falha no `configure`).
     */
    fun acceptedFor(wanted: Int): Set<Int> = when (wanted) {
        HIGH -> setOf(HIGH, CONSTRAINED_HIGH)
        BASELINE -> setOf(BASELINE, CONSTRAINED_BASELINE)
        else -> setOf(wanted) // MAIN: sem variante constrained
    }

    /**
     * Primeiro perfil de [preferred] que [supported] cobre (ou `null`).
     * Puro: os ints vêm das `CodecCapabilities.profileLevels` do aparelho.
     */
    fun pickProfile(supported: List<Int>, preferred: List<Int> = H264Profile.preferred): Int? =
        preferred.firstOrNull { wanted -> acceptedFor(wanted).any { supported.contains(it) } }

    fun name(id: Int): String = when (id) {
        BASELINE -> "baseline"
        MAIN -> "main"
        HIGH -> "high"
        CONSTRAINED_BASELINE -> "constrained_baseline"
        CONSTRAINED_HIGH -> "constrained_high"
        else -> "p$id"
    }
}

/**
 * Candidato a encoder, medido do aparelho (`MediaCodecList`):
 * [canEncode] = `CodecCapabilities.isFormatSupported` para a resolução/fps/
 * bitrate pedidos; [supportedProfiles] = ids de `profileLevels`;
 * [supportsInputSurface] = `colorFormats` contém `COLOR_FormatSurface` —
 * a câmera renderiza na superfície de entrada do encoder, que é
 * **obrigatória**. Não existe constante `FEATURE_INPUT_SURFACE` na API
 * (conf. AOSP main e 8.1); a decisão do framework é pelo formato de cor.
 * Padrão `true`: os fakes de teste JVM não simulam o recurso.
 */
data class EncoderCandidate(
    val name: String,
    val canEncode: Boolean,
    val supportedProfiles: List<Int>,
    val supportsInputSurface: Boolean = true,
)

/**
 * Candidatos em ordem de tentativa (cartão F2-T08 — **não fixar nome**):
 * 1. só entram os capazes (`canEncode`), com perfil preferido
 *    (High > Main > Baseline) e com **input surface** (a câmera renderiza
 *    na superfície de entrada — sem ela `createInputSurface()` lança);
 * 2. ordena pelo melhor perfil; empate → ordem estável do `MediaCodecList`.
 * Puro (testável em JVM com fakes); vazio = nenhum encoder H.264 utilizável.
 */
fun rankEncoders(
    candidates: List<EncoderCandidate>,
    preferred: List<Int> = H264Profile.preferred,
): List<EncoderCandidate> {
    val ranked = ArrayList<Pair<Int, EncoderCandidate>>()
    for (c in candidates) {
        if (!c.canEncode || !c.supportsInputSurface) continue
        val profile = H264Profile.pickProfile(c.supportedProfiles, preferred) ?: continue
        ranked.add(preferred.indexOf(profile) to c) // >= 0 por construção
    }
    ranked.sortBy { it.first } // estável: empate preserva a ordem da lista
    return ranked.map { it.second }
}

/** Primeiro de [rankEncoders]; `null` = nenhum encoder H.264 utilizável. */
fun pickEncoder(
    candidates: List<EncoderCandidate>,
    preferred: List<Int> = H264Profile.preferred,
): EncoderCandidate? = rankEncoders(candidates, preferred).firstOrNull()
