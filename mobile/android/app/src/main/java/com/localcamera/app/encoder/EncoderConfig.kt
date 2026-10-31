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
 * bitrate pedidos; [supportedProfiles] = ids de `profileLevels`.
 */
data class EncoderCandidate(
    val name: String,
    val canEncode: Boolean,
    val supportedProfiles: List<Int>,
)

/**
 * Regra de escolha do encoder (cartão F2-T08 — **não fixar nome**):
 * 1. só entram candidatos capazes (`canEncode`);
 * 2. vence o que atende o **melhor perfil** da ordem preferida
 *    (High > Main > Baseline);
 * 3. empate de perfil → quem aparece antes no `MediaCodecList`.
 * Puro (testável em JVM com fakes); `null` = nenhum encoder H.264 utilizável.
 */
fun pickEncoder(
    candidates: List<EncoderCandidate>,
    preferred: List<Int> = H264Profile.preferred,
): EncoderCandidate? {
    var best: EncoderCandidate? = null
    var bestRank = Int.MAX_VALUE
    for (c in candidates) {
        if (!c.canEncode) continue
        val profile = H264Profile.pickProfile(c.supportedProfiles, preferred) ?: continue
        val rank = preferred.indexOf(profile) // >= 0 por construção de pickProfile
        if (rank < bestRank) { // empate de perfil → o mais cedo na lista vence
            best = c
            bestRank = rank
        }
    }
    return best
}
