package com.localcamera.app.plugin

import android.Manifest
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.util.Log
import com.getcapacitor.JSObject
import com.getcapacitor.Plugin
import com.getcapacitor.PluginCall
import com.getcapacitor.PluginMethod
import com.getcapacitor.annotation.CapacitorPlugin
import com.getcapacitor.annotation.Permission
import com.getcapacitor.annotation.PermissionCallback
import com.localcamera.app.camera.CapabilitiesProbe
import com.localcamera.app.service.StreamBus
import com.localcamera.app.service.StreamEvent
import com.localcamera.app.service.StreamingService
import com.localcamera.app.store.PairingStore
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import org.json.JSONObject
import java.net.URLEncoder

/**
 * Plugin Capacitor `LocalCamera` (cartão F2-T13) — a ponte JS ⇄ serviço de
 * streaming. O plugin **nunca instancia o [StreamingService]**: `connect`/
 * `disconnect` delegam por intent ([StreamingService.startStreaming]/
 * [stopStreaming]); os eventos vêm do [StreamBus] (intra-processo).
 *
 * ## Métodos (JS → Kotlin)
 * - `getCapabilities()` → `{caps}` (sonda F2-T06, JSON `caps` do HELLO);
 * - `connect({hosts, port, token})` (pareamento, QR/deep link) **ou**
 *   `connect({pcId})` (PC conhecido — hosts/porta do `PairingStore`, auth por
 *   `deviceKey`; F2-T05);
 * - `disconnect()` → para a sessão (BYE no lado do serviço);
 * - `getState()` → `{state}`: último JSON `STATE` com `rev` (F2-T11).
 * - `getKnownPcs()`/`forgetPc({pcId})` → lista/esquecimento de PCs com chave
 *   persistida; `deviceKey` continua exclusivamente no Android Keystore.
 *
 * ## Eventos (Kotlin → JS)
 * - `connectionChanged {state, timeoutS?, pcName?}` — `idle|connecting|pending|
 *   streaming|reconnecting|error` (máquina §12);
 * - `error {code}` — **só códigos** (`denied_<r>`, `bye_<r>`, `app_not_visible`,
 *   ...); nenhum texto da rede; a UI mapeia para i18n;
 * - `stats {fps, kbps, queueMs, rtt}` — o serviço já agrega a cada 500 ms
 *   (2 Hz, `StreamSession.statsLoop`); aqui só reencaminho — o `AppProvider`
 *   (JS) re-força o teto de 1 a cada 500 ms;
 * - `stateChanged {state}` — JSON `STATE` com `rev` (retain até consumido: o
 *   webview pode recarregar durante a transmissão).
 *
 * ## Segredos
 * `token`/`deviceKey` **nunca** entram em log, evento ou `toString` (regra de
 * ouro do repo). A URI montada em `connect` contém o token: só viaja no intent
 * interno (mesmo processo) e nunca é logada.
 *
 * ## Registro
 * A anotação [@CapacitorPlugin] é o contrato; `capacitor.plugins.json` (asset)
 * é **regenerado** pelo `cap sync`, que escaneia o pacote local `@localcamera/plugin`
 * (ver `app/package.json` + stub em `app/android/@localcamera/plugin/`).
 */
@CapacitorPlugin(
    name = "LocalCamera",
    permissions = [
        Permission(alias = "camera", strings = [Manifest.permission.CAMERA]),
        Permission(alias = "mic", strings = [Manifest.permission.RECORD_AUDIO]),
        Permission(alias = "notifications", strings = [Manifest.permission.POST_NOTIFICATIONS]),
    ],
)
class LocalCameraPlugin : Plugin() {

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private var observing = false

    override fun load() {
        if (observing) return
        observing = true
        scope.launch { collectEvents() }
    }

    override fun handleOnDestroy() {
        scope.cancel()
    }

    // ------------------------------ eventos (Kotlin → JS) ------------------------------

    private suspend fun collectEvents() {
        StreamBus.events.collect { ev ->
            when (ev) {
                StreamEvent.Idle ->
                    notifyConnection(EVENT_IDLE)
                StreamEvent.Connecting ->
                    notifyConnection(EVENT_CONNECTING)
                is StreamEvent.Pending ->
                    notifyListeners(
                        EVENT_CONNECTION_CHANGED,
                        JSObject()
                            .put(KEY_STATE, EVENT_PENDING)
                            .put(KEY_TIMEOUT_S, ev.timeoutS),
                    )
                is StreamEvent.Streaming ->
                    notifyListeners(
                        EVENT_CONNECTION_CHANGED,
                        JSObject()
                            .put(KEY_STATE, EVENT_STREAMING)
                            .put(KEY_PC_NAME, ev.pcName),
                    )
                StreamEvent.Reconnecting ->
                    notifyConnection(EVENT_RECONNECTING)
                // `error` tem código próprio (evento `error` abaixo); o estado
                // "error" da conexão chega junto com o evento de erro.
                is StreamEvent.Error -> {
                    // `connectionChanged{state:"error"}` (máquina §12) + `error{code}`.
                    notifyConnection(EVENT_ERROR)
                    notifyListeners(EVENT_ERROR, JSObject().put(KEY_CODE, ev.code))
                }
                is StreamEvent.Stats ->
                    // Já agregado a 500 ms pela fonte (StreamSession.statsLoop);
                    // o teto final (UI) é re-forçado no AppProvider JS.
                    notifyListeners(
                        EVENT_STATS,
                        JSObject()
                            .put(KEY_FPS, ev.fps)
                            .put(KEY_KBPS, ev.kbps)
                            .put(KEY_QUEUE_MS, ev.queueMs)
                            .put(KEY_RTT, ev.rttMs),
                    )
                is StreamEvent.StateChanged ->
                    // retainUntilConsumed: webview recarregada recebe o último
                    // estado ao re-registrar o listener.
                    // `true` = retainUntilConsumed (método Java: argumento
                    // posicional — Kotlin não aceita named args p/ Java).
                    notifyListeners(EVENT_STATE_CHANGED, JSObject().put(KEY_STATE, ev.json), true)
            }
        }
    }

    private fun notifyConnection(state: String) {
        notifyListeners(EVENT_CONNECTION_CHANGED, JSObject().put(KEY_STATE, state))
    }

    // ------------------------------ métodos (JS → Kotlin) ------------------------------

    /** Estado real das permissões runtime solicitadas pela UI de primeira execução. */
    @PluginMethod
    fun getPermissions(call: PluginCall) {
        call.resolve(
            JSObject()
                .put(PERMISSION_CAMERA, permissionStatus(PERMISSION_CAMERA))
                .put(PERMISSION_MIC, permissionStatus(PERMISSION_MIC))
                .put(PERMISSION_NOTIFICATIONS, permissionStatus(PERMISSION_NOTIFICATIONS)),
        )
    }

    /** Solicita uma permissão por vez; a tela pede câmera antes de permitir continuar. */
    @PluginMethod
    fun requestPermission(call: PluginCall) {
        val kind = call.getString(KEY_PERMISSION_KIND)
        val alias = permissionAlias(kind)
        if (alias == null) {
            call.reject(ERROR_INVALID_PERMISSION_KIND)
            return
        }
        if (alias == PERMISSION_NOTIFICATIONS && Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
            call.resolve(JSObject().put(KEY_PERMISSION_STATUS, PERMISSION_GRANTED))
            return
        }
        requestPermissionForAlias(alias, call, CALLBACK_PERMISSION_RESULT)
    }

    @PermissionCallback
    fun permissionRequestResult(call: PluginCall) {
        val kind = call.getString(KEY_PERMISSION_KIND)
        if (permissionAlias(kind) == null) {
            call.reject(ERROR_INVALID_PERMISSION_KIND)
            return
        }
        call.resolve(JSObject().put(KEY_PERMISSION_STATUS, permissionStatus(kind!!)))
    }

    /** Abre as configurações do app quando o Android não permite novo prompt. */
    @PluginMethod
    fun openPermissionSettings(call: PluginCall) {
        try {
            val intent = Intent(
                Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                Uri.fromParts("package", getContext().packageName, null),
            )
            getActivity().startActivity(intent)
            call.resolve()
        } catch (_: Exception) {
            call.reject(ERROR_SETTINGS_UNAVAILABLE)
        }
    }

    /** `{caps}` — JSON da sonda do aparelho (F2-T06). */
    @PluginMethod
    fun getCapabilities(call: PluginCall) {
        try {
            val caps = JSONObject(CapabilitiesProbe.probe(getContext()))
            call.resolve(JSObject().put(KEY_CAPS, caps))
        } catch (t: Throwable) {
            log("sonda de capabilities falhou (${t::class.simpleName})")
            call.reject(ERROR_CAPS)
        }
    }

    /**
     * Inicia a sessão via intent ([StreamingService.startStreaming]).
     * `connect({hosts, port, token})` | `connect({pcId})`.
     * Recusas: `invalid_args`, `unknown_pc`, `no_device_key` (**códigos**).
     */
    @PluginMethod
    fun connect(call: PluginCall) {
        when (val r = buildConnectUri(call)) {
            is ConnectSpec.ConnectUri -> {
                // URI contém o token (segredo): nunca logar.
                log(if (r.pcId != null) "connect pc=${r.pcId}" else "connect hosts=${r.hostCount}")
                StreamingService.startStreaming(getContext(), r.uri)
                call.resolve()
            }
            is ConnectSpec.ConnectError -> call.reject(r.code)
        }
    }

    /** Para a sessão (BYE + teardown no serviço). Idempotente. */
    @PluginMethod
    fun disconnect(call: PluginCall) {
        StreamingService.stopStreaming(getContext())
        call.resolve()
    }

    /** `{state}` — último JSON `STATE` com `rev` (`""` sem sessão ativa). */
    @PluginMethod
    fun getState(call: PluginCall) {
        call.resolve(JSObject().put(KEY_STATE, StreamBus.lastStateJson))
    }

    /** `{pcs}` sem segredos; só inclui PCs cuja `deviceKey` existe no Keystore. */
    @PluginMethod
    fun getKnownPcs(call: PluginCall) = KnownPcPluginBridge.getKnownPcs(getContext(), call, ::log)

    /** Esquece o PC e apaga a chave correspondente no Keystore. */
    @PluginMethod
    fun forgetPc(call: PluginCall) = KnownPcPluginBridge.forgetPc(getContext(), call, ::log)

    // ------------------------------ montagem da URI ------------------------------

    private sealed interface ConnectSpec {
        data class ConnectUri(val uri: String, val pcId: String?, val hostCount: Int) : ConnectSpec
        data class ConnectError(val code: String) : ConnectSpec
    }

    private fun buildConnectUri(call: PluginCall): ConnectSpec {
        val pcId = call.getString(KEY_PC_ID)
        val hasPairingFields = call.getArray(KEY_HOSTS) != null ||
            call.getInt(KEY_PORT) != null || call.getString(KEY_TOKEN) != null
        if (pcId != null && !hasPairingFields) return knownPcUri(pcId)

        val hosts = call.getArray(KEY_HOSTS)?.let { arr ->
            try {
                arr.toList<String>()
            } catch (t: Exception) {
                emptyList()
            }
        } ?: emptyList()
        val port = call.getInt(KEY_PORT) ?: 0
        // O token do pareamento é minúsculo no parser (F2-T05a); input em
        // maiúsculo (tela Manual) é o mesmo código → normaliza.
        val token = call.getString(KEY_TOKEN)?.lowercase().orEmpty()
        return pairingUri(hosts, port, token, pc = pcId, name = null)
    }

    /** PC conhecido (F2-T05): hosts/porta do [PairingStore]; auth por `deviceKey`. */
    private fun knownPcUri(pcId: String): ConnectSpec {
        val store = PairingStore(getContext())
        val pc = store.getPc(pcId) ?: return ConnectSpec.ConnectError(ERROR_UNKNOWN_PC)
        if (store.deviceKey(pcId) == null) return ConnectSpec.ConnectError(ERROR_NO_DEVICE_KEY)
        // A autenticação vai por `deviceKey` (HelloBuilder prefere-a quando
        // existe); o `token` é placeholder apenas para satisfazer o parser da
        // URI (que exige 32 hex) — nunca é usado para autenticar aqui.
        return pairingUri(pc.hosts, pc.port, PLACEHOLDER_TOKEN, pc = pcId, name = pc.name)
    }

    /** Valida o esqueleto e monta `localcamera://connect?…` (ordem fixa, §4). */
    private fun pairingUri(
        hosts: List<String>,
        port: Int,
        token: String,
        pc: String?,
        name: String?,
    ): ConnectSpec {
        if (hosts.isEmpty() || hosts.size > MAX_HOSTS) return ConnectSpec.ConnectError(ERROR_INVALID_ARGS)
        if (port !in 1..MAX_PORT) return ConnectSpec.ConnectError(ERROR_INVALID_ARGS)
        if (!TOKEN_RE.matches(token)) return ConnectSpec.ConnectError(ERROR_INVALID_ARGS)
        if (pc != null && !PC_ID_RE.matches(pc)) return ConnectSpec.ConnectError(ERROR_INVALID_ARGS)

        val q = StringBuilder("v=$PROTOCOL_V&hosts=${hosts.joinToString(",")}&port=$port&token=$token")
        if (pc != null) q.append("&pc=").append(pc)
        if (!name.isNullOrEmpty()) q.append("&name=").append(URLEncoder.encode(name, "UTF-8"))
        return ConnectSpec.ConnectUri("$URI_PREFIX$q", pcId = pc, hostCount = hosts.size)
    }

    private fun log(msg: String) {
        Log.i(TAG, msg)
    }

    private fun permissionAlias(kind: String?): String? = when (kind) {
        PERMISSION_CAMERA, PERMISSION_MIC, PERMISSION_NOTIFICATIONS -> kind
        else -> null
    }

    private fun permissionStatus(kind: String): String {
        if (kind == PERMISSION_NOTIFICATIONS && Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
            return PERMISSION_GRANTED
        }
        return getPermissionState(kind)?.toString() ?: PERMISSION_PROMPT
    }

    private companion object {
        const val TAG = "LocalCameraPlugin"

        // Eventos (nomes fixos do contrato JS ⇄ Kotlin, cartão F2-T13).
        const val EVENT_CONNECTION_CHANGED = "connectionChanged"
        const val EVENT_STATE_CHANGED = "stateChanged"
        const val EVENT_STATS = "stats"
        const val EVENT_ERROR = "error"

        // Valores de `connectionChanged.state` (máquina §12 — mesmo vocabulário JS).
        const val EVENT_IDLE = "idle"
        const val EVENT_CONNECTING = "connecting"
        const val EVENT_PENDING = "pending"
        const val EVENT_STREAMING = "streaming"
        const val EVENT_RECONNECTING = "reconnecting"

        // Chaves do payload.
        const val KEY_STATE = "state"
        const val KEY_TIMEOUT_S = "timeoutS"
        const val KEY_PC_NAME = "pcName"
        const val KEY_CODE = "code"
        const val KEY_FPS = "fps"
        const val KEY_KBPS = "kbps"
        const val KEY_QUEUE_MS = "queueMs"
        const val KEY_RTT = "rtt"
        const val KEY_CAPS = "caps"
        const val KEY_PERMISSION_KIND = "kind"
        const val KEY_PERMISSION_STATUS = "status"

        const val PERMISSION_CAMERA = "camera"
        const val PERMISSION_MIC = "mic"
        const val PERMISSION_NOTIFICATIONS = "notifications"
        const val PERMISSION_GRANTED = "granted"
        const val PERMISSION_PROMPT = "prompt"
        const val CALLBACK_PERMISSION_RESULT = "permissionRequestResult"

        // Opções de `connect`.
        const val KEY_HOSTS = "hosts"
        const val KEY_PORT = "port"
        const val KEY_TOKEN = "token"
        const val KEY_PC_ID = "pcId"

        // Códigos de recusa (estáveis; a UI mapeia por código).
        const val ERROR_INVALID_ARGS = "invalid_args"
        const val ERROR_UNKNOWN_PC = "unknown_pc"
        const val ERROR_NO_DEVICE_KEY = "no_device_key"
        const val ERROR_CAPS = "caps_failed"
        const val ERROR_INVALID_PERMISSION_KIND = "invalid_permission_kind"
        const val ERROR_SETTINGS_UNAVAILABLE = "permission_settings_unavailable"

        // Montagem da URI (mesmas constantes do parser F2-T05a).
        const val URI_PREFIX = "localcamera://connect?"
        const val PROTOCOL_V = 1
        const val MAX_HOSTS = 4
        const val MAX_PORT = 65535
        val TOKEN_RE = Regex("^[0-9a-f]{32}$")
        val PC_ID_RE = Regex("^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")
        /** 32 hex nulos: satisfaz o parser; a auth real é a `deviceKey`. */
        const val PLACEHOLDER_TOKEN = "00000000000000000000000000000000"
    }
}
