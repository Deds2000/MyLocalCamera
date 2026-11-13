package com.localcamera.app.camera

/**
 * Modo da sessão de streaming (decisão do usuário, 2026-11-02): o melhor modo
 * 16:9 suportado pela câmera e ≤ 1080p — configurado no encoder **e** na
 * câmera ao iniciar a sessão, e anunciado em `STREAM_CONFIG`/`STATE`.
 *
 * Contexto: o HAL do Moto G (5) rejeita a superfície de entrada do encoder em
 * 1920×1080 (set válido termina em 1280×720) → sessão sem vídeo no default
 * antigo; o Samsung aceita 1080p. A seleção por probe resolve os dois sem
 * fixar resolução por aparelho. Kotlin puro: nenhum tipo Android aqui
 * (testável em JVM).
 */
data class SessionMode(val width: Int, val height: Int, val fps: Int)

/**
 * Seleção do modo de sessão a partir dos modos reportados pela sonda
 * ([CapabilitiesProbe.sessionModes]). Puro (JVM):
 * - só 16:9 estrito (a pipeline do app é 16:9 — `CameraXBackend` usa
 *   `RATIO_16_9_FALLBACK_AUTO_STRATEGY`);
 * - dentro do teto (default do cartão: 1920×1080);
 * - maior área vence; empate → primeiro na lista (ordem estável);
 * - lista vazia (ou nada elegível) → `null`: o chamador falha fechado (não
 *   anuncia um modo sem suporte — decisão do usuário 2026-11-03).
 */
object SessionModePicker {

    /** Teto da seleção (CHEATSHEET: 1080p30). */
    val DEFAULT = SessionMode(1920, 1080, 30)

    fun pick(modes: List<SessionMode>, ceiling: SessionMode = DEFAULT): SessionMode? =
        modes
            .filter { it.fps > 0 && it.width in 1..ceiling.width && it.height in 1..ceiling.height && is16x9(it.width, it.height) }
            .maxByOrNull { it.width.toLong() * it.height }

    /** 16:9 estrito, em inteiros (evita erro de ponto flutuante). */
    fun is16x9(width: Int, height: Int): Boolean = width > 0 && height > 0 && width * 9 == height * 16
}
