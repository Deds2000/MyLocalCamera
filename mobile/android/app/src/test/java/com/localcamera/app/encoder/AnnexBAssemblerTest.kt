package com.localcamera.app.encoder

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * [AnnexBAssembler] (cartão F2-T08): buffers de saída do `MediaCodec` →
 * access units. Fronteiras: buffer seguinte começando com start code, AUD
 * (NAL 9) interno, AU cortado no meio do NAL, SC de 4 bytes partido entre
 * buffers e o último AU em [AnnexBAssembler.flush].
 */
class AnnexBAssemblerTest {

    private fun bytes(vararg xs: Int): ByteArray = ByteArray(xs.size) { i -> xs[i].toByte() }

    private val aud = bytes(0, 0, 1, 0x09, 0x10)
    private val sps = bytes(0, 0, 1, 0x67, 0x28, 0x01, 0x02)
    private val pps = bytes(0, 0, 1, 0x68, 0x88, 0x01)
    private val idr = bytes(0, 0, 1, 0x65, 0xAA, 0xBB, 0xCC)
    private val slice = bytes(0, 0, 1, 0x41, 0xDD, 0xEE, 0xFF)
    private val idrAu = sps + pps + idr
    private val sliceAu = slice

    /** Compara lists de ByteArray por conteúdo (o assembler devolve cópias). */
    private fun assertUnits(expected: List<ByteArray>, actual: List<ByteArray>) {
        assertEquals(expected.size, actual.size)
        expected.forEachIndexed { i, e -> assertArrayEquals("AU $i", e, actual[i]) }
    }

    @Test
    fun `1 AU por buffer - o AU so sai quando o proximo buffer chega`() {
        val a = AnnexBAssembler()
        assertUnits(emptyList(), a.add(idrAu))
        assertUnits(listOf(idrAu), a.add(sliceAu))
        assertUnits(listOf(sliceAu), a.flush())
    }

    @Test
    fun `AU cortado no meio do NAL e reassemblado no buffer seguinte`() {
        val idrHalf1 = idr.copyOfRange(0, 5) // 00 00 01 65 AA BB
        val idrHalf2 = idr.copyOfRange(5, idr.size) // CC
        val a = AnnexBAssembler()
        assertUnits(emptyList(), a.add(sps + pps + idrHalf1))
        assertUnits(emptyList(), a.add(idrHalf2))
        assertUnits(listOf(idrAu), a.add(sliceAu))
        assertUnits(listOf(sliceAu), a.flush())
    }

    @Test
    fun `AUD interno divide AUs no mesmo buffer`() {
        val a = AnnexBAssembler()
        assertUnits(listOf(aud + sps + pps + idr), a.add(aud + sps + pps + idr + aud + sliceAu))
        assertUnits(listOf(aud + sliceAu), a.flush())
    }

    @Test
    fun `2 AUs sem AUD no mesmo buffer viram um unico AU`() {
        val slice2 = bytes(0, 0, 1, 0x41, 0x77)
        val a = AnnexBAssembler()
        assertUnits(emptyList(), a.add(sliceAu + slice2))
        assertUnits(listOf(sliceAu + slice2), a.add(slice2)) // fronteira só no buffer seguinte
        assertUnits(listOf(slice2), a.flush())
    }

    @Test
    fun `start code de 4 bytes partido entre buffers e renormalizado`() {
        val unit4 = bytes(0, 0, 0, 1, 0x65, 0xAA, 0xBB)
        val a = AnnexBAssembler()
        assertUnits(emptyList(), a.add(unit4.copyOfRange(0, 2))) // [00 00] sem SC
        assertUnits(emptyList(), a.add(unit4.copyOfRange(2, unit4.size))) // [00 01 65 AA BB]
        assertUnits(listOf(bytes(0, 0, 1, 0x65, 0xAA, 0xBB)), a.add(sliceAu))
        assertUnits(listOf(sliceAu), a.flush())
    }

    @Test
    fun `start code de 4 bytes inteiro chega normalizado para 3 bytes`() {
        val a = AnnexBAssembler()
        val au1 = bytes(0, 0, 0, 1, 0x65, 0xAA, 0xBB)
        assertUnits(emptyList(), a.add(au1))
        assertUnits(listOf(bytes(0, 0, 1, 0x65, 0xAA, 0xBB)), a.add(sliceAu))
        assertUnits(listOf(sliceAu), a.flush())
    }

    @Test
    fun `lixo antes do primeiro start code e descartado`() {
        val a = AnnexBAssembler()
        val out = a.add(bytes(0x55, 0x56, 0x57) + aud + sps + pps + idr + aud + sliceAu)
        assertUnits(listOf(aud + sps + pps + idr), out)
        assertUnits(listOf(aud + sliceAu), a.flush())
    }

    @Test
    fun `sem fronteira o pendente limpa no cap e resincroniza`() {
        val a = AnnexBAssembler()
        val junk = ByteArray(9 * 1024 * 1024 + 1) { 0x55 }
        assertUnits(emptyList(), a.add(junk))
        assertUnits(emptyList(), a.add(idrAu))
        assertUnits(listOf(idrAu), a.add(sliceAu))
        assertUnits(listOf(sliceAu), a.flush())
    }
}
