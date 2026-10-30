package com.localcamera.app.camera

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Hint de rotação (cartão F2-T07, passo 3) — fórmula medida no S3:
 * sensor traseiro 90° → retrato 90°, paisagem 0°
 * (`spikes/s3/SpikeTest.kt` e `docs/spikes/S3.md` §Encoders e orientação).
 */
class RotationHintTest {

    @Test
    fun `traseira sensor 90 retrato e paisagem (valores S3)`() {
        assertEquals(90, RotationHint.forCamera(Facing.BACK, 90, 0))
        assertEquals(0, RotationHint.forCamera(Facing.BACK, 90, 90))
        assertEquals(270, RotationHint.forCamera(Facing.BACK, 90, 180))
        assertEquals(180, RotationHint.forCamera(Facing.BACK, 90, 270))
    }

    @Test
    fun `frontal sensor 270 (típico) fórmula espelhada`() {
        assertEquals(90, RotationHint.forCamera(Facing.FRONT, 270, 0))
        assertEquals(180, RotationHint.forCamera(Facing.FRONT, 270, 90))
        assertEquals(270, RotationHint.forCamera(Facing.FRONT, 270, 180))
        assertEquals(0, RotationHint.forCamera(Facing.FRONT, 270, 270))
    }

    @Test
    fun `resultado sempre 0 90 180 ou 270`() {
        val validos = setOf(0, 90, 180, 270)
        for (sensor in intArrayOf(0, 90, 180, 270)) {
            for (aparelho in intArrayOf(0, 90, 180, 270)) {
                assertTrue(RotationHint.forCamera(Facing.BACK, sensor, aparelho) in validos)
                assertTrue(RotationHint.forCamera(Facing.FRONT, sensor, aparelho) in validos)
            }
        }
    }
}
