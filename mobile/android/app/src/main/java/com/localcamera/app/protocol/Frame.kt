package com.localcamera.app.protocol

/** Magic little-endian; na rede aparece como `43 4C`. */
const val MAGIC: Int = 0x4C43

/** Bytes do magic na rede (little-endian de [MAGIC]). */
val MAGIC_BYTES: ByteArray = byteArrayOf(0x43, 0x4C)

/** Tamanho do cabeçalho de quadro, em bytes. */
const val HEADER_LEN: Int = 18

/** Limite padrão de payload: 8 MiB (`0x00800000`). */
const val MAX_PAYLOAD: Int = 8 * 1024 * 1024

/** Limite do payload de HELLO: 64 KiB. */
const val MAX_HELLO_PAYLOAD: Int = 64 * 1024

/**
 * Tipos de mensagem do protocolo (tabela do CHEATSHEET).
 * Código fora da tabela vira [UNKNOWN]; o byte original fica em [Frame.typeCode].
 */
enum class MessageType(val code: Int) {
    HELLO(0x01),
    WELCOME(0x02),
    AUDIO_HELLO(0x03),
    PENDING(0x04),
    AUDIO_TICKET_REQUEST(0x05),
    AUDIO_TICKET(0x06),
    VIDEO(0x10),
    AUDIO(0x11),
    STREAM_CONFIG(0x12),
    SET(0x20),
    STATE(0x21),
    PING(0x30),
    PONG(0x31),
    STATS(0x32),
    REQUEST_KEYFRAME(0x40),
    BYE(0x7F),
    /** Código fora da tabela: a conexão continua; o quadro é entregue ao chamador. */
    UNKNOWN(-1);

    companion object {
        fun fromCode(code: Int): MessageType =
            entries.firstOrNull { it.code == code } ?: UNKNOWN
    }
}

/** Um quadro completo decodificado (cabeçalho de 18 bytes + payload). */
class Frame(
    /** Byte bruto de tipo; para tipos desconhecidos, o código original. */
    val typeCode: Int,
    val flags: Int,
    val stream: Int,
    /** Microssegundos (u64 lido como Long; use `toULong().toString()` para exibir sem sinal). */
    val ptsUs: Long,
    val payload: ByteArray,
) {
    /** [UNKNOWN] para código fora da tabela — nunca é erro de decodificação. */
    val type: MessageType get() = MessageType.fromCode(typeCode)

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is Frame) return false
        return typeCode == other.typeCode &&
            flags == other.flags &&
            stream == other.stream &&
            ptsUs == other.ptsUs &&
            payload.contentEquals(other.payload)
    }

    override fun hashCode(): Int {
        var result = typeCode
        result = 31 * result + flags
        result = 31 * result + stream
        result = 31 * result + ptsUs.hashCode()
        result = 31 * result + payload.contentHashCode()
        return result
    }

    override fun toString(): String =
        "Frame(${type.name}, typeCode=0x%02X, flags=$flags, stream=$stream, " +
            "ptsUs=${ptsUs.toULong()}, payload=${payload.size}B)".format(typeCode)
}

internal fun readU32Le(b: ByteArray, off: Int): Int =
    (b[off].toInt() and 0xFF) or
        ((b[off + 1].toInt() and 0xFF) shl 8) or
        ((b[off + 2].toInt() and 0xFF) shl 16) or
        ((b[off + 3].toInt() and 0xFF) shl 24)

internal fun readU64Le(b: ByteArray, off: Int): Long =
    readU32Le(b, off).toLong() or (readU32Le(b, off + 4).toLong() shl 32)

internal fun writeU32Le(out: ByteArray, off: Int, v: Int) {
    out[off] = (v and 0xFF).toByte()
    out[off + 1] = ((v ushr 8) and 0xFF).toByte()
    out[off + 2] = ((v ushr 16) and 0xFF).toByte()
    out[off + 3] = ((v ushr 24) and 0xFF).toByte()
}

internal fun writeU64Le(out: ByteArray, off: Int, v: Long) {
    writeU32Le(out, off, v.toInt())
    writeU32Le(out, off + 4, (v ushr 32).toInt())
}
