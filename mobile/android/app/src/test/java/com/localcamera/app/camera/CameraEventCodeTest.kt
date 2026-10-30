package com.localcamera.app.camera

import androidx.camera.core.CameraState
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * [CameraEventCode.fromCameraState] (F2-T07): mapeamento dos erros do
 * CameraX para códigos estáveis do app (sem texto de UI — a UI faz o i18n).
 */
class CameraEventCodeTest {

    @Test
    fun `mapeia todos os erros do camerax`() {
        assertEquals(
            CameraEventCode.CAMERA_IN_USE,
            CameraEventCode.fromCameraState(CameraState.ERROR_CAMERA_IN_USE),
        )
        assertEquals(
            CameraEventCode.MAX_CAMERAS_IN_USE,
            CameraEventCode.fromCameraState(CameraState.ERROR_MAX_CAMERAS_IN_USE),
        )
        assertEquals(
            CameraEventCode.CAMERA_RECOVERABLE,
            CameraEventCode.fromCameraState(CameraState.ERROR_OTHER_RECOVERABLE_ERROR),
        )
        assertEquals(
            CameraEventCode.STREAM_CONFIG,
            CameraEventCode.fromCameraState(CameraState.ERROR_STREAM_CONFIG),
        )
        assertEquals(
            CameraEventCode.CAMERA_DISABLED,
            CameraEventCode.fromCameraState(CameraState.ERROR_CAMERA_DISABLED),
        )
        assertEquals(
            CameraEventCode.CAMERA_FATAL,
            CameraEventCode.fromCameraState(CameraState.ERROR_CAMERA_FATAL_ERROR),
        )
        assertEquals(
            CameraEventCode.DO_NOT_DISTURB,
            CameraEventCode.fromCameraState(CameraState.ERROR_DO_NOT_DISTURB_MODE_ENABLED),
        )
        assertEquals(
            CameraEventCode.CAMERA_REMOVED,
            CameraEventCode.fromCameraState(CameraState.ERROR_CAMERA_REMOVED),
        )
    }

    @Test
    fun `codigo desconhecido vira fatal`() {
        assertEquals(CameraEventCode.CAMERA_FATAL, CameraEventCode.fromCameraState(0x1234))
    }
}
