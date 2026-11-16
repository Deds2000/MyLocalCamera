package com.localcamera.app.camera

import androidx.camera.core.CameraState

/** Decisão pura de estado CameraX → evento do app; não acessa hardware nem Android. */
internal object CameraStateReducer {

    data class Transition(
        val sawOpenState: Boolean,
        val startedEmitted: Boolean,
        val event: CameraEvent? = null,
        val shouldReset: Boolean = false,
    )

    fun reduce(
        type: CameraState.Type,
        errorCode: Int?,
        running: Boolean,
        sawOpenState: Boolean,
        startedEmitted: Boolean,
    ): Transition {
        if (errorCode != null) {
            return Transition(
                sawOpenState = sawOpenState,
                startedEmitted = false,
                event = CameraEvent.Error(CameraEventCode.fromCameraState(errorCode)),
                shouldReset = true,
            )
        }

        return when (type) {
            CameraState.Type.OPENING -> Transition(
                sawOpenState = true,
                startedEmitted = startedEmitted,
            )
            CameraState.Type.OPEN -> Transition(
                sawOpenState = true,
                startedEmitted = true,
                event = if (startedEmitted) null else CameraEvent.Started,
            )
            CameraState.Type.CLOSED -> {
                val interrupted = running && sawOpenState
                Transition(
                    sawOpenState = sawOpenState,
                    startedEmitted = if (interrupted) false else startedEmitted,
                    event = if (interrupted) CameraEvent.Error(CameraEventCode.CAMERA_CLOSED) else null,
                    shouldReset = interrupted,
                )
            }
            else -> Transition(
                sawOpenState = sawOpenState,
                startedEmitted = startedEmitted,
            ) // PENDING_OPEN, CLOSING
        }
    }
}
