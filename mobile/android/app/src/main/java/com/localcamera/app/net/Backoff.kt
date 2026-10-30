package com.localcamera.app.net

/**
 * Backoff exponencial de reconexão (F2-T09): **1 s, 2 s, 4 s, 8 s, 8 s, …**
 *
 * Puro (sem relógio): [next] devolve o próximo intervalo **e** avança;
 * [reset] (após sessão bem-sucedida) volta ao primeiro valor. Quem espera
 * é o chamador, com relógio injetável (`Clock`).
 */
class Backoff(
    private val initialMs: Long = DEFAULT_INITIAL_MS,
    private val factor: Int = DEFAULT_FACTOR,
    private val maxMs: Long = DEFAULT_MAX_MS,
) {

    private var currentMs = initialMs

    init {
        require(initialMs > 0) { "initialMs deve ser > 0" }
        require(factor >= 2) { "factor deve ser >= 2" }
        require(maxMs >= initialMs) { "maxMs deve ser >= initialMs" }
    }

    /** Próximo intervalo em ms: 1000, 2000, 4000, 8000, 8000, … (teto [maxMs]). */
    fun next(): Long {
        val value = currentMs
        currentMs = minOf(currentMs * factor, maxMs)
        return value
    }

    /** Volta ao primeiro valor — chamado após sessão bem-sucedida. */
    fun reset() {
        currentMs = initialMs
    }

    companion object {
        /** Primeira espera de reconexão (1 s, F2-T09). */
        const val DEFAULT_INITIAL_MS = 1000L

        /** Multiplicador (2 → 1, 2, 4, 8 s). */
        const val DEFAULT_FACTOR = 2

        /** Teto do backoff (8 s, F2-T09). */
        const val DEFAULT_MAX_MS = 8000L
    }
}
