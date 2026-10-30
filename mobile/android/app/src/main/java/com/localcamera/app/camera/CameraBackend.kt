package com.localcamera.app.camera

import android.view.Surface
import androidx.camera.core.CameraState

/**
 * Eventos da câmera (cartão F2-T07): `started`, `error(codigo)`, `rotated`.
 * Os códigos são identificador de máquina (sem texto de UI): a UI mapeia
 * [Error.code] para chave de i18n. O mapeamento do `CameraState` → código
 * fica em [CameraEventCode.fromCameraState] (dentro do pacote `camera/`).
 */
sealed interface CameraEvent {

    /** Câmera aberta entregando quadros na superfície do encoder. */
    data object Started : CameraEvent

    /** Falha/interrupção (outro app abriu a câmera, chamada, etc. — FT-2.04/2.05 em F2-T14). */
    data class Error(val code: String) : CameraEvent

    /** Hint de rotação atualizado (0/90/180/270). */
    data class Rotated(val hint: Int) : CameraEvent
}

/** Códigos de [CameraEvent.Error] — estáveis, sem frase de UI. */
object CameraEventCode {
    const val CAMERA_IN_USE = "camera_in_use"
    const val MAX_CAMERAS_IN_USE = "max_cameras_in_use"
    const val CAMERA_RECOVERABLE = "camera_recoverable"
    const val STREAM_CONFIG = "stream_config"
    const val CAMERA_DISABLED = "camera_disabled"
    const val CAMERA_FATAL = "camera_fatal"
    const val DO_NOT_DISTURB = "do_not_disturb"
    const val CAMERA_REMOVED = "camera_removed"
    const val PERMISSION_DENIED = "permission_denied"
    const val CAMERA_UNAVAILABLE = "camera_unavailable"
    const val CAMERA_CLOSED = "camera_closed"
    const val START_FAILED = "start_failed"

    /**
     * Mapeia código de erro do CameraX (`CameraState.ERROR_*`) para código
     * estável do app. Puro (testável em JVM): as constantes chegam
     * inlinadas como `Int`.
     */
    fun fromCameraState(code: Int): String = when (code) {
        CameraState.ERROR_CAMERA_IN_USE -> CAMERA_IN_USE
        CameraState.ERROR_MAX_CAMERAS_IN_USE -> MAX_CAMERAS_IN_USE
        CameraState.ERROR_OTHER_RECOVERABLE_ERROR -> CAMERA_RECOVERABLE
        CameraState.ERROR_STREAM_CONFIG -> STREAM_CONFIG
        CameraState.ERROR_CAMERA_DISABLED -> CAMERA_DISABLED
        CameraState.ERROR_CAMERA_FATAL_ERROR -> CAMERA_FATAL
        CameraState.ERROR_DO_NOT_DISTURB_MODE_ENABLED -> DO_NOT_DISTURB
        CameraState.ERROR_CAMERA_REMOVED -> CAMERA_REMOVED
        else -> CAMERA_FATAL
    }
}

/**
 * Backend de câmera (cartão F2-T07): a UI/sessão **nunca** importam
 * CameraX — usam só esta interface. Implementação única CameraX
 * (veredito S3, modo single; o dual via Camera2 é da Fase 7).
 *
 * Contrato: [start] abre a câmera e alimenta [start.outputSurface] com
 * quadros 1080p30; [stop] fecha; [switchTo] troca a câmera ativa mantendo
 * o stream; eventos por [onEvent]; hint de rotação por [onRotationHint].
 * Chamados pela `StreamingService` (F2-T12), que detém o [LifecycleOwner].
 */
interface CameraBackend {

    /** Abre a câmera segundo [config] e entrega quadros na Surface do encoder. */
    fun start(config: CameraConfig, outputSurface: Surface)

    /** Fecha a câmera e libera o hardware. Idempotente. */
    fun stop()

    /** Troca a câmera ativa (back/front) mantendo o stream; ignora se igual ou inativo. */
    fun switchTo(facing: Facing)

    /** Registra listener do hint de rotação (0/90/180/270). */
    fun onRotationHint(listener: (Int) -> Unit)

    /** Registra listener de eventos ([CameraEvent.Started]/[CameraEvent.Error]/[CameraEvent.Rotated]). */
    fun onEvent(listener: (CameraEvent) -> Unit)
}
