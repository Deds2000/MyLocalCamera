package com.localcamera.app.encoder

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * [H264]/[EncodedFrame.flags] (cartão F2-T08): análise pura de NALs em
 * Annex-B (mesma premissa do `lc-testkit::annexb` do lado Rust) e as flags
 * do cabeçalho do protocolo (bit0 keyframe, bit1 SPS/PPS).
 */
class H264Test {

    private fun bytes(vararg xs: Int): ByteArray = ByteArray(xs.size) { i -> xs[i].toByte() }

    private val sps = bytes(0, 0, 1, 0x67, 0x28, 0x01, 0x02)
    private val pps = bytes(0, 0, 1, 0x68, 0x88, 0x01)
    private val idr = bytes(0, 0, 1, 0x65, 0xAA, 0xBB, 0xCC)
    private val slice = bytes(0, 0, 1, 0x41, 0xDD, 0xEE)
    private val idrAu = sps + pps + idr
    private val sliceAu = slice

    @Test
    fun `nalTypes com start code de 3 bytes`() {
        assertEquals(listOf(7, 8, 5), H264.nalTypes(idrAu))
        assertEquals(listOf(1), H264.nalTypes(sliceAu))
    }

    @Test
    fun `nalTypes com start code de 4 bytes`() {
        val unit4 = bytes(0, 0, 0, 1, 0x65, 0xAA, 0xBB)
        assertEquals(listOf(5), H264.nalTypes(unit4))
    }

    @Test
    fun `nalTypes sem start code devolve vazio`() {
        val emptyTypes: List<Int> = emptyList()
        assertEquals(emptyTypes, H264.nalTypes(bytes(0x65, 0xAA)))
        assertEquals(emptyTypes, H264.nalTypes(ByteArray(0)))
    }

    @Test
    fun `isIdr so para AU com slice IDR (NAL 5)`() {
        assertTrue(H264.isIdr(idrAu))
        assertTrue(H264.isIdr(idr))
        assertFalse(H264.isIdr(sps + pps))
        assertFalse(H264.isIdr(sliceAu))
    }

    @Test
    fun `hasParamSets exige SPS e PPS no mesmo AU`() {
        assertTrue(H264.hasParamSets(idrAu))
        assertFalse(H264.hasParamSets(sps))
        assertFalse(H264.hasParamSets(pps))
        assertFalse(H264.hasParamSets(sliceAu))
    }

    @Test
    fun `flagsFor IDR com SPS e PPS e 0x03`() {
        assertEquals(EncodedFrame.FLAG_KEYFRAME or EncodedFrame.FLAG_PARAM_SETS, H264.flagsFor(idrAu))
    }

    @Test
    fun `flagsFor IDR sem SPS e PPS e 0x01`() {
        assertEquals(EncodedFrame.FLAG_KEYFRAME, H264.flagsFor(idr))
    }

    @Test
    fun `flagsFor SPS e PPS sem IDR e 0x02`() {
        assertEquals(EncodedFrame.FLAG_PARAM_SETS, H264.flagsFor(sps + pps))
    }

    @Test
    fun `flagsFor slice comum e 0x00`() {
        assertEquals(0, H264.flagsFor(sliceAu))
    }

    @Test
    fun `prependParamSets cola os param sets na frente do AU`() {
        val out = H264.prependParamSets(sps + pps, idr)
        assertArrayEquals(sps + pps + idr, out)
        assertTrue(H264.isIdr(out))
        assertTrue(H264.hasParamSets(out))
        assertEquals(EncodedFrame.FLAG_KEYFRAME or EncodedFrame.FLAG_PARAM_SETS, H264.flagsFor(out))
    }
}
