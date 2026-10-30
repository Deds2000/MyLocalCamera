package com.localcamera.app.protocol

/** Resultado de [FrameReader.feed]. */
sealed class ReaderResult {
    /** Quadros completos emitidos; o resto fica pendente no leitor. */
    data class Ok(val frames: List<Frame>) : ReaderResult()

    /** Leitor envenenado (BadMagic/TooLarge): o chamador deve fechar a conexão. */
    data class Error(val error: CodecError) : ReaderResult()
}

/**
 * Leitor incremental de quadros (mesmo desenho do Rust `lc-protocol`):
 * buffer interno, emite frames completos e o resto fica pendente
 * ([pendingBytes]).
 *
 * Tipo desconhecido **não** é erro: sai como [Frame] com [MessageType.UNKNOWN].
 * Depois de um erro o leitor fica **envenenado**: todo [feed] seguinte devolve
 * o mesmo erro e o buffer é descartado (o chamador fecha a conexão).
 */
class FrameReader(private val maxPayload: Int = MAX_PAYLOAD) {

    private var buf = ByteArray(64)
    private var len = 0
    private var poisoned: CodecError? = null

    /** Bytes ainda não consumidos (cabeçalho/payload incompletos). */
    val pendingBytes: Int
        get() = len

    /** Alimenta bytes e devolve todos os quadros completos. */
    fun feed(data: ByteArray): ReaderResult {
        val err = poisoned
        if (err != null) return ReaderResult.Error(err)

        val newLen = len + data.size
        if (newLen > buf.size) {
            var cap = buf.size
            while (cap < newLen) cap *= 2
            buf = buf.copyOf(cap)
        }
        data.copyInto(buf, len)
        len = newLen

        val frames = ArrayList<Frame>()
        while (len >= HEADER_LEN) {
            if (buf[0] != MAGIC_BYTES[0] || buf[1] != MAGIC_BYTES[1]) {
                return poison(CodecError.BadMagic)
            }
            val typeCode = buf[2].toInt() and 0xFF
            val flags = buf[3].toInt() and 0xFF
            val stream = buf[4].toInt() and 0xFF
            val ptsUs = readU64Le(buf, 6)
            // Valida `length` ANTES de consumir qualquer payload.
            val length = readU32Le(buf, 14)
            if (length > maxPayload) {
                return poison(CodecError.TooLarge(length.toLong()))
            }
            val total = HEADER_LEN + length
            if (len < total) break // payload incompleto: espera mais bytes

            val payload = buf.copyOfRange(HEADER_LEN, total)
            val rest = len - total
            if (rest > 0) System.arraycopy(buf, total, buf, 0, rest)
            len = rest
            frames.add(Frame(typeCode, flags, stream, ptsUs, payload))
        }
        return ReaderResult.Ok(frames)
    }

    /** Envenena o leitor: guarda o erro e descarta o buffer. */
    private fun poison(err: CodecError): ReaderResult {
        poisoned = err
        len = 0
        return ReaderResult.Error(err)
    }
}
