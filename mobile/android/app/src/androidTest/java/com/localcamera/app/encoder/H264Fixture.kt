package com.localcamera.app.encoder

/** Helpers para a fixture H.264 Annex-B usada pelos testes instrumentados. */
internal object H264Fixture {

    /** Divide o byte stream em access units iniciadas por NAL AUD (tipo 9). */
    fun splitAccessUnits(bytes: ByteArray): List<ByteArray> {
        val starts = ArrayList<Int>()
        var i = 0
        while (i + 3 < bytes.size) {
            val startCodeSize = when {
                i + 3 < bytes.size && bytes[i] == 0.toByte() && bytes[i + 1] == 0.toByte() &&
                    bytes[i + 2] == 0.toByte() && bytes[i + 3] == 1.toByte() -> 4
                bytes[i] == 0.toByte() && bytes[i + 1] == 0.toByte() && bytes[i + 2] == 1.toByte() -> 3
                else -> 0
            }
            if (startCodeSize == 0) {
                i++
                continue
            }
            val header = i + startCodeSize
            if (header < bytes.size && (bytes[header].toInt() and 0x1f) == NAL_AUD) starts.add(i)
            i = header + 1
        }
        require(starts.isNotEmpty() && starts.first() == 0) {
            "fixture Annex-B deve começar com um NAL AUD"
        }
        return starts.indices.map { index ->
            val end = starts.getOrNull(index + 1) ?: bytes.size
            bytes.copyOfRange(starts[index], end)
        }
    }

    private const val NAL_AUD = 9
}
