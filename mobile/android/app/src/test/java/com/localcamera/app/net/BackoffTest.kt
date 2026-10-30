package com.localcamera.app.net

import org.junit.Assert.assertEquals
import org.junit.Test

/** UT-2.05 — Backoff 1-2-4-8-8… e reset após sucesso. */
class BackoffTest {

    @Test
    fun ut205_sequencia12488ESeTeta() {
        val b = Backoff()
        assertEquals(
            listOf(1000L, 2000L, 4000L, 8000L, 8000L, 8000L, 8000L),
            List(7) { b.next() },
        )
    }

    @Test
    fun ut205_resetVoltaAoPrimeiroValor() {
        val b = Backoff()
        repeat(4) { b.next() } // 1, 2, 4, 8
        b.reset()
        assertEquals(listOf(1000L, 2000L, 4000L), List(3) { b.next() })
    }

    @Test
    fun ut205_resetApósSessãoBemSucedidaResetaNoLaço() {
        // Comportamento usado pelo laço: sessão bem-sucedida → reset → 1 s.
        val b = Backoff()
        b.next(); b.next() // falhas: 1 s, 2 s
        assertEquals(4000L, b.next())
        b.reset() // sessão bem-sucedida
        assertEquals(1000L, b.next())
    }
}
