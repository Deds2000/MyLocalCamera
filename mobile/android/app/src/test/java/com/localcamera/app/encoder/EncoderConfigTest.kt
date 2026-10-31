package com.localcamera.app.encoder

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

/** [EncoderConfig] (cartão F2-T08): padrão do cartão e validação dos campos. */
class EncoderConfigTest {

    @Test
    fun `padrao do cartao 1080p30 CBR 8Mbps keyframe 1s`() {
        val cfg = EncoderConfig()
        assertEquals(1920, cfg.width)
        assertEquals(1080, cfg.height)
        assertEquals(30, cfg.fps)
        assertEquals(8000, cfg.bitrateKbps)
        assertEquals(1, cfg.keyframeIntervalS)
    }

    @Test
    fun `valores invalidos sao rejeitados`() {
        assertThrows(IllegalArgumentException::class.java) { EncoderConfig(width = 0) }
        assertThrows(IllegalArgumentException::class.java) { EncoderConfig(height = -1) }
        assertThrows(IllegalArgumentException::class.java) { EncoderConfig(fps = 0) }
        assertThrows(IllegalArgumentException::class.java) { EncoderConfig(bitrateKbps = 0) }
        assertThrows(IllegalArgumentException::class.java) { EncoderConfig(keyframeIntervalS = 0) }
    }
}
