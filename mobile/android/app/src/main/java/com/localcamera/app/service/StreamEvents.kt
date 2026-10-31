package com.localcamera.app.service

import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import java.util.concurrent.atomic.AtomicInteger

/**
 * API pública do serviço de streaming (F2-T12) — consumida pelo plugin
 * Capacitor (F2-T13). Só **códigos e dados estruturados**: nenhum texto
 * da rede, nenhum segredo (`token`/`deviceKey` jamais trafegam por aqui).
 *
 * Estados da conexão (protocolo §12, máquina de estados do celular):
 * `idle → connecting → (pending) → streaming → reconnecting → streaming | idle`.
 */
enum class ConnectionState(val code: String) {
    IDLE("idle"),
    CONNECTING("connecting"),
    PENDING("pending"),
    STREAMING("streaming"),
    RECONNECTING("reconnecting"),
    ERROR("error");
}

/** Eventos de [StreamSession]/[StreamingService] para a UI (via [StreamBus]). */
sealed interface StreamEvent {

    /** Sessão encerrada (usuário parou ou erro definitivo, protocolo §12). */
    data object Idle : StreamEvent

    /** Sessão iniciada; conexão/handshake em curso. */
    data object Connecting : StreamEvent

    /** PC aguardando aprovação no desktop (`PENDING{timeoutS}`). */
    data class Pending(val timeoutS: Int) : StreamEvent

    /** `WELCOME` aceito: `STATE` + `STREAM_CONFIG` enviados, vídeo fluindo. */
    data class Streaming(val pcName: String) : StreamEvent

    /** Conexão perdida; reconexão com backoff (1, 2, 4, 8 s — F2-T09). */
    data object Reconnecting : StreamEvent

    /** Erro identificado por **código** (a UI mapeia para i18n). */
    data class Error(val code: String) : StreamEvent

    /**
     * Estatísticas agregadas para a UI — no máximo a cada 500 ms (2 Hz,
     * cartão F2-T12/CHEATSHEET). [rttMs] = 0 até a medição de PING/PONG
     * ser exposta pela `PrimaryConnection` (F2-T09, pendência registrada).
     */
    data class Stats(val fps: Int, val kbps: Int, val queueMs: Long, val rttMs: Long) : StreamEvent

    /** Estado compartilhado atual (JSON `STATE` com `rev`) — UI `stateChanged`. */
    data class StateChanged(val json: String) : StreamEvent

    /** Estado de conexão que o evento impõe; `null` = não altera (Stats/StateChanged). */
    val state: ConnectionState?
        get() = when (this) {
            Idle -> ConnectionState.IDLE
            Connecting -> ConnectionState.CONNECTING
            is Pending -> ConnectionState.PENDING
            is Streaming -> ConnectionState.STREAMING
            Reconnecting -> ConnectionState.RECONNECTING
            is Error -> ConnectionState.ERROR
            else -> null
        }
}

/** Códigos de erro do serviço (estáveis; prefixo `<origem>_<código do protocolo>`). */
object StreamErrorCodes {

    /** O app não estava visível: start recusado (armadilha §6 do cartão). */
    const val APP_NOT_VISIBLE = "app_not_visible"

    /** URI de pareamento ausente/inválida no intent de start. */
    const val BAD_URI = "bad_uri"

    /** Permissão de câmera negada (mesmo código de `CameraEventCode.PERMISSION_DENIED`). */
    const val CAMERA_PERMISSION_DENIED = "permission_denied"

    /** `WELCOME` recusado: `denied_<reason>` (`bad_token`, `busy`, `revoked`, ...). */
    fun denied(reason: String): String = "denied_$reason"

    /** `BYE` do PC: `bye_<reason>`. */
    fun bye(reason: String): String = "bye_$reason"
}

/**
 * Bus de eventos **dentro do processo** (mesmo processo webview → plugin):
 * o plugin Capacitor (F2-T13) observa [events] e repassa para o JS.
 * [tryEmit] não bloqueia: eventos vindos de threads de rede nunca podem
 * travar o pipeline; se o consumidor estiver lento, os mais antigos caem.
 */
object StreamBus {

    private val flow = MutableSharedFlow<StreamEvent>(
        extraBufferCapacity = 256,
        onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )

    val events: Flow<StreamEvent> = flow.asSharedFlow()

    /** Último estado de conexão (para consumidores que entram tarde / `getState`). */
    @Volatile
    var lastState: ConnectionState = ConnectionState.IDLE
        private set

    /** Último JSON de estado (para `getState` do plugin). */
    @Volatile
    var lastStateJson: String = ""
        private set

    fun emit(ev: StreamEvent) {
        ev.state?.let { lastState = it }
        if (ev is StreamEvent.StateChanged) lastStateJson = ev.json
        flow.tryEmit(ev)
    }
}

/**
 * App **visível** = alguma activity `resumed` (armadilha §6 do cartão: a
 * câmera/FGS de câmera não pode ser iniciada com o app em segundo plano).
 * [MainActivity] alimenta o contador em `onResume`/`onPause`; o
 * `StreamingService` consulta [isVisible] antes de aceitar o start.
 * (`@JvmStatic`: chamado do Java do `MainActivity`.)
 */
internal object AppForeground {

    private val resumed = AtomicInteger(0)

    @JvmStatic
    fun onActivityResumed() {
        resumed.incrementAndGet()
    }

    @JvmStatic
    fun onActivityPaused() {
        if (resumed.decrementAndGet() < 0) resumed.set(0)
    }

    @JvmStatic
    fun isVisible(): Boolean = resumed.get() > 0
}
