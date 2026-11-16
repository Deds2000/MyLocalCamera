package com.localcamera.app.camera

import androidx.camera.core.CameraState
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CameraStateReducerTest {

    @Test
    fun `closed inicial antes de abrir e ignorado`() {
        val transition = CameraStateReducer.reduce(
            type = CameraState.Type.CLOSED,
            errorCode = null,
            running = true,
            sawOpenState = false,
            startedEmitted = false,
        )

        assertEquals(null, transition.event)
        assertFalse(transition.shouldReset)
        assertFalse(transition.sawOpenState)
    }

    @Test
    fun `closed depois de open reporta camera fechada`() {
        val opened = CameraStateReducer.reduce(
            type = CameraState.Type.OPEN,
            errorCode = null,
            running = true,
            sawOpenState = false,
            startedEmitted = false,
        )
        assertEquals(CameraEvent.Started, opened.event)

        val closed = CameraStateReducer.reduce(
            type = CameraState.Type.CLOSED,
            errorCode = null,
            running = true,
            sawOpenState = opened.sawOpenState,
            startedEmitted = opened.startedEmitted,
        )

        assertEquals(CameraEvent.Error(CameraEventCode.CAMERA_CLOSED), closed.event)
        assertTrue(closed.shouldReset)
    }

    @Test
    fun `camera em uso por outro app reporta codigo correto`() {
        val transition = CameraStateReducer.reduce(
            type = CameraState.Type.CLOSED,
            errorCode = CameraState.ERROR_CAMERA_IN_USE,
            running = true,
            sawOpenState = false,
            startedEmitted = false,
        )

        assertEquals(CameraEvent.Error(CameraEventCode.CAMERA_IN_USE), transition.event)
        assertTrue(transition.shouldReset)
    }
}
