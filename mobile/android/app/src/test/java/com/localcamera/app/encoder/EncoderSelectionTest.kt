package com.localcamera.app.encoder

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * [pickEncoder]/[H264Profile.pickProfile] (cartão F2-T08): regra de escolha
 * pura do encoder — sem nome fixo, melhor perfil (High > Main > Baseline),
 * empate → ordem do `MediaCodecList`.
 */
class EncoderSelectionTest {

    private fun c(name: String, profiles: List<Int>, canEncode: Boolean = true) =
        EncoderCandidate(name, canEncode, profiles)

    // ------------------------- H264Profile -------------------------

    @Test
    fun `constrained high atende o pedido de high`() {
        assertEquals(H264Profile.HIGH, H264Profile.pickProfile(listOf(H264Profile.CONSTRAINED_HIGH)))
    }

    @Test
    fun `high vence mesmo se baseline aparece primeiro na lista`() {
        assertEquals(
            H264Profile.HIGH,
            H264Profile.pickProfile(listOf(H264Profile.BASELINE, H264Profile.HIGH)),
        )
    }

    @Test
    fun `sem high cai para main e depois baseline`() {
        assertEquals(
            H264Profile.MAIN,
            H264Profile.pickProfile(listOf(H264Profile.BASELINE, H264Profile.MAIN)),
        )
        assertEquals(
            H264Profile.BASELINE,
            H264Profile.pickProfile(listOf(H264Profile.BASELINE)),
        )
    }

    @Test
    fun `lista sem nenhum perfil preferido devolve null`() {
        assertNull(H264Profile.pickProfile(emptyList()))
        assertNull(H264Profile.pickProfile(listOf(9)))
    }

    @Test
    fun `nome dos perfis para STREAM_CONFIG`() {
        assertEquals("high", H264Profile.name(H264Profile.HIGH))
        assertEquals("main", H264Profile.name(H264Profile.MAIN))
        assertEquals("baseline", H264Profile.name(H264Profile.BASELINE))
        assertEquals("constrained_high", H264Profile.name(H264Profile.CONSTRAINED_HIGH))
        assertEquals("p131072", H264Profile.name(131072))
    }

    // ------------------------- pickEncoder -------------------------

    @Test
    fun `o melhor perfil vence mesmo aparecendo depois na lista`() {
        val pick = pickEncoder(
            listOf(
                c("a", listOf(H264Profile.BASELINE)),
                c("b", listOf(H264Profile.HIGH)),
            ),
        )
        assertEquals("b", pick?.name)
    }

    @Test
    fun `empate de perfil ganha quem aparece antes no MediaCodecList`() {
        val pick = pickEncoder(
            listOf(
                c("a", listOf(H264Profile.HIGH)),
                c("b", listOf(H264Profile.CONSTRAINED_HIGH)),
            ),
        )
        assertEquals("a", pick?.name)
    }

    @Test
    fun `candidato incapaz de codificar fica de fora`() {
        val pick = pickEncoder(
            listOf(
                c("a", listOf(H264Profile.HIGH), canEncode = false),
                c("b", listOf(H264Profile.MAIN)),
            ),
        )
        assertEquals("b", pick?.name)
    }

    @Test
    fun `candidato sem perfil utilizavel fica de fora`() {
        assertNull(pickEncoder(listOf(c("a", listOf(9)))))
    }

    @Test
    fun `lista vazia devolve null`() {
        assertNull(pickEncoder(emptyList()))
    }
}
