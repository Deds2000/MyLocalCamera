package com.localcamera.app.protocol

/** Serializa quadros: cabeçalho de 18 bytes little-endian + payload. */
object FrameWriter {

    /**
     * Serializa [frame] para bytes prontos para o socket.
     * @throws CodecError.TooLarge se o payload exceder 8 MiB.
     */
    fun write(frame: Frame): ByteArray {
        if (frame.payload.size > MAX_PAYLOAD) {
            throw CodecError.TooLarge(frame.payload.size.toLong())
        }
        val out = ByteArray(HEADER_LEN + frame.payload.size)
        out[0] = MAGIC_BYTES[0]
        out[1] = MAGIC_BYTES[1]
        out[2] = frame.typeCode.toByte()
        out[3] = frame.flags.toByte()
        out[4] = frame.stream.toByte()
        out[5] = 0 // reserved
        writeU64Le(out, 6, frame.ptsUs)
        writeU32Le(out, 14, frame.payload.size)
        frame.payload.copyInto(out, HEADER_LEN)
        return out
    }
}
