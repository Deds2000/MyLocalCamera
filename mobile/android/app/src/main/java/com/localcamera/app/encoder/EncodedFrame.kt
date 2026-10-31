package com.localcamera.app.encoder

/**
 * Quadro H.264 pronto para a fila (cartão F2-T08): 1 access unit em
 * Annex-B (start codes de 3 bytes) + `ptsUs` do relógio monotônico +
 * [flags] do cabeçalho do protocolo (CHEATSHEET: bit0 = keyframe,
 * bit1 = tem SPS/PPS).
 */
data class EncodedFrame(
    val ptsUs: Long,
    val annexb: ByteArray,
    val flags: Int,
) {
    /** Bit 0 do cabeçalho: keyframe (IDR). */
    val isKeyframe: Boolean get() = flags and FLAG_KEYFRAME != 0

    /** Bit 1 do cabeçalho: o payload contém SPS+PPS. */
    val hasParamSets: Boolean get() = flags and FLAG_PARAM_SETS != 0

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is EncodedFrame) return false
        return ptsUs == other.ptsUs && flags == other.flags && annexb.contentEquals(other.annexb)
    }

    override fun hashCode(): Int =
        31 * (31 * (ptsUs.hashCode() * 31 + annexb.contentHashCode()) + flags)

    override fun toString(): String =
        "EncodedFrame(ptsUs=$ptsUs, ${annexb.size}B, " +
            "keyframe=$isKeyframe, spsPps=$hasParamSets)"

    companion object {
        /** Bit 0 de `flags` (mesmo valor de [com.localcamera.app.net.SendQueue.KEYFRAME_FLAG]). */
        const val FLAG_KEYFRAME: Int = 0x01
        /** Bit 1 de `flags`: payload contém SPS/PPS. */
        const val FLAG_PARAM_SETS: Int = 0x02
    }
}

/**
 * Análise H.264 pura (sem Android — testável em JVM). Operam sobre um
 * "unit" = NAL(s) em Annex-B, com start codes de 3 ou 4 bytes.
 *
 * O byte stream Annex-B nunca contém `00 00 01` dentro de um NAL (regra de
 * start-code emulation prevention), então o start code delimita NALs de
 * forma inequívoca — mesma premissa do lado Rust (`lc-testkit::annexb`).
 */
object H264 {

    /** Tipos de NAL presentes em [unit] (bits 1–5 do 1º byte após cada start code). */
    fun nalTypes(unit: ByteArray): List<Int> {
        val out = ArrayList<Int>()
        var i = 0
        while (i + 3 < unit.size) {
            if (unit[i] == 0.toByte() && unit[i + 1] == 0.toByte() && unit[i + 2] == 1.toByte()) {
                if (i + 3 < unit.size) out.add(unit[i + 3].toInt() and 0x1F)
                i += 3
            } else {
                i++
            }
        }
        return out
    }

    /** NAL 5 = slice IDR → access unit é keyframe. */
    fun isIdr(unit: ByteArray): Boolean = 5 in nalTypes(unit)

    /** NAL 7 = SPS · NAL 8 = PPS. */
    fun hasSps(unit: ByteArray): Boolean = 7 in nalTypes(unit)

    fun hasPps(unit: ByteArray): Boolean = 8 in nalTypes(unit)

    /** SPS **e** PPS no mesmo access unit (requisito do protocolo §10). */
    fun hasParamSets(unit: ByteArray): Boolean = hasSps(unit) && hasPps(unit)

    /** [flags] do quadro a partir do conteúdo: bit0 IDR · bit1 SPS+PPS. */
    fun flagsFor(unit: ByteArray): Int =
        (if (isIdr(unit)) EncodedFrame.FLAG_KEYFRAME else 0) or
            (if (hasParamSets(unit)) EncodedFrame.FLAG_PARAM_SETS else 0)

    /**
     * Anexa os parameter sets guardados ([spsPps]) à frente do [unit]
     * (armadilha §6: encoders que não pré-preendem SPS/PPS nos keyframes).
     * [spsPps] já começa com start code de 3 bytes.
     */
    fun prependParamSets(spsPps: ByteArray, unit: ByteArray): ByteArray {
        val out = ByteArray(spsPps.size + unit.size)
        spsPps.copyInto(out)
        unit.copyInto(out, spsPps.size)
        return out
    }
}

/**
 * Reassemblagem dos buffers de saída do `MediaCodec` em **access units**
 * Annex-B (pura — testável em JVM). O encoder pode entregar 1 AU por
 * buffer (comportamento do OMX.qcom validado no S3), vários AUs num buffer
 * (delimitados por AUD) ou um AU cortado no meio de um NAL.
 *
 * Regras de fronteira de AU:
 * - buffer novo começando com start code → o AU pendente está completo
 *   (emite e começa um novo); buffer sem start code inicial → continuação
 *   do NAL anterior (mesmo AU);
 * - dentro do AU, um NAL 9 (AUD) que **não** é o primeiro NAL delimita o
 *   próximo AU (mesma premissa do lado Rust: AUs começam em AUD);
 * - sem AUD, um buffer pode conter no máximo 1 AU (limitação documentada —
 *   2 AUs sem AUD no mesmo buffer viram 1 AU, ainda decodificável);
 * - fragmentos sem start code (start code de 4 bytes partido entre buffers)
 *   ficam pendurados; sem start code além de [MAX_PENDING], o pendente é
 *   descartado e o próximo start code re-sincroniza;
 * - start codes de 4 bytes chegam normalizados para 3 bytes (o zero extra
 *   é descartado; mesma detecção `00 00 01` do lado Rust).
 */
class AnnexBAssembler {

    /** AU em montagem (começa com start code de 3 bytes, salvo fragmentos). */
    private var au = ByteArray(0)

    /** Acrescenta um buffer de saída; devolve os access units completos (na ordem). */
    fun add(bytes: ByteArray): List<ByteArray> {
        val out = ArrayList<ByteArray>()
        if (au.isNotEmpty() && findStartCode(au, 0) >= 0 && startsWithStartCode(bytes)) {
            out += au // AU pendente completo (o próximo buffer abriu um NAL novo)
            au = ByteArray(0)
        }
        if (au.isEmpty()) {
            val sc = findStartCode(bytes, 0)
            au = if (sc >= 0) bytes.copyOfRange(sc, bytes.size) else bytes
        } else {
            au = au + bytes // continuação de NAL (ou de SC de 4 bytes partido)
        }
        out += extract()
        if (au.size > MAX_PENDING) au = ByteArray(0) // corrompido: resincroniza
        return out
    }

    /** Fim do stream (stop): extraí o que restou (o último AU, se houver). */
    fun flush(): List<ByteArray> {
        val out = extract().toMutableList()
        val sc = findStartCode(au, 0)
        if (sc >= 0) out += au.copyOfRange(sc, au.size)
        au = ByteArray(0)
        return out
    }

    /** Normaliza o AU (descarta zeros antes do 1º SC) e divide em AUs nos AUDs. */
    private fun extract(): List<ByteArray> {
        val out = ArrayList<ByteArray>()
        if (au.isEmpty()) return out
        val sc = findStartCode(au, 0)
        if (sc < 0) return out // ainda sem start code (fragmento pendente)
        if (sc > 0) au = au.copyOfRange(sc, au.size)
        var i = 0
        var nals = 0
        while (true) {
            val next = findStartCode(au, i + 4)
            if (next < 0) break // último NAL ainda incompleto — fica pendente
            nals++
            if (nals > 0 && nalTypeAt(next) == NAL_AUD) { // AUD não inicial = novo AU
                out += au.copyOfRange(0, next)
                au = au.copyOfRange(next, au.size)
                i = 0
            } else {
                i = next
            }
        }
        return out
    }

    /** Tipo do NAL logo após o start code em [sc] (bits 1–5), ou `null`. */
    private fun nalTypeAt(sc: Int): Int? =
        if (sc + 3 < au.size) au[sc + 3].toInt() and 0x1F else null

    private fun startsWithStartCode(b: ByteArray): Boolean =
        b.size >= 3 && b[0] == 0.toByte() && b[1] == 0.toByte() && b[2] == 1.toByte()

    /** Posição do 1º start code (`00 00 01`) em [b] a partir de [from], ou -1. */
    private fun findStartCode(b: ByteArray, from: Int): Int {
        var i = from.coerceAtLeast(0)
        while (i + 2 < b.size) {
            if (b[i] == 0.toByte() && b[i + 1] == 0.toByte() && b[i + 2] == 1.toByte()) return i
            i++
        }
        return -1
    }

    private companion object {
        /** NAL 9 = AUD (Audio/Access Unit Delimiter). */
        private const val NAL_AUD = 9
        /** Sem fronteira para além disso: pendente considerado corrompido. */
        private const val MAX_PENDING = 9 * 1024 * 1024
    }
}
