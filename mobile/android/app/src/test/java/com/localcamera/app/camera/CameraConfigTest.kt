package com.localcamera.app.camera

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

/** [CameraConfig]/[Facing] (cartão F2-T07): padrão do cartão e códigos do protocolo. */
class CameraConfigTest {

    @Test
    fun `padrao do cartao 1080p30`() {
        val cfg = CameraConfig(Facing.BACK)
        assertEquals(1920, cfg.width)
        assertEquals(1080, cfg.height)
        assertEquals(30, cfg.fps)
    }

    @Test
    fun `codigos do protocolo back e front`() {
        assertEquals(Facing.BACK, Facing.fromCode("back"))
        assertEquals(Facing.FRONT, Facing.fromCode("front"))
        assertEquals("back", Facing.BACK.code)
        assertEquals("front", Facing.FRONT.code)
    }

    @Test
    fun `codigo desconhecido devolve null`() {
        assertNull(Facing.fromCode("external"))
        assertNull(Facing.fromCode("Back"))
        assertNull(Facing.fromCode(""))
    }

    @Test
    fun `resolucao e fps invalidos sao rejeitados`() {
        assertThrows(IllegalArgumentException::class.java) { CameraConfig(Facing.BACK, width = 0) }
        assertThrows(IllegalArgumentException::class.java) { CameraConfig(Facing.BACK, height = -1) }
        assertThrows(IllegalArgumentException::class.java) { CameraConfig(Facing.BACK, fps = 0) }
    }
}
