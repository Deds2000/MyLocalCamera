package com.localcamera.app.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry
import com.localcamera.app.R
import com.localcamera.app.camera.CameraEventCode
import com.localcamera.app.store.PairingStore
import com.localcamera.app.uri.PairingUri
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

/**
 * `LifecycleService` em primeiro plano (cartão F2-T12): possui a
 * [StreamSession] (câmera + encoder + conexão + fila + estado) e a mantém
 * com o app em segundo plano; notificação no canal `streaming` com a ação
 * "Parar" (a notificação some; o app segue aberto). Raiz de composição de
 * F2-T07/08/09/10/11.
 *
 * ## Requisitos de foreground service — targetSdk 36 (pesquisa do passo 1)
 * Fontes (developer.android.com, consultado 30/10/2026): página de tipos de
 * FGS (`develop/background-work/services/fgs`, seção "service-types") e
 * `about/versions/14/changes/fgs-types-required`:
 * 1. **Android 14+ (target 34+):** o manifest precisa declarar
 *    `android:foregroundServiceType` no `<service>` **e** a permissão
 *    `FOREGROUND_SERVICE_<TYPE>` correspondente (`camera`, `microphone`);
 *    sem isso, o start de FGS quebra (`MissingForegroundServiceTypeException`).
 * 2. **Android 12+:** depois de `startForegroundService()`, `startForeground(id,
 *    notif)` deve ser chamado em ~5 s ou o sistema mata o app
 *    (`ForegroundServiceDidNotStartInTimeException`) — por isso o start vai
 *    ao foreground **antes** das validações (falha → [stopStream] logo depois).
 * 3. **Android 12+:** iniciar FGS com o app em segundo plano exige exceção;
 *    caso contrário `startForegroundService()` lança
 *    `BackgroundServiceStartNotAllowedException` no chamador. Em 14+ o tipo
 *    `camera` não pode ser criado do background — daí a guarda [AppForeground]
 *    (armadilha §6 do cartão): start sem app visível → `error{app_not_visible}`.
 * 4. **Android 13+ (API 33):** `POST_NOTIFICATIONS` é permissão de **runtime**;
 *    sem ela o FGS funciona, mas a notificação não aparece. O pedido de
 *    permissão é da UI/plugin (F2-T13) — o serviço só declara no manifest.
 * 5. **Android 15+:** timeout de 6 h vale só para o tipo `dataSync` (não se
 *    aplica a `camera`); a câmera FGS continua exigindo app visível/exceção.
 * 6. **minSdk 24:** `startForegroundService()` é API 26+ →
 *    [ContextCompat.startForegroundService] (fallback `startService`).
 *
 * `WAKE_LOCK` está declarado (cartão) para uso posterior; o FGS já mantém o
 * processo vivo na Fase 2.
 */
class StreamingService : Service() {

    /** Escopo dos jobs da sessão (main + SupervisorJob; cancelado em [onDestroy]). */
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

    /** Owner de lifecycle da sessão (RESUMED durante o stream, CREATED fora). */
    private val owner = ServiceLifecycleOwner()

    private val store: PairingStore by lazy { PairingStore(this) }
    private var session: StreamSession? = null
    private var currentPc = ""

    override fun onBind(intent: Intent): android.os.IBinder? = null // serviço não vinculado

    override fun onCreate() {
        super.onCreate()
        ensureChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP -> stopStream()
            else -> startFromIntent(intent)
        }
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        session?.stop()
        scope.cancel() // mata o teardown em curso; o fim do processo limpa o resto
        super.onDestroy()
    }

    // ------------------------------ start ------------------------------

    private fun startFromIntent(intent: Intent?) {
        if (session != null) {
            log("start ignorado: sessão já ativa")
            return
        }
        // (2) acima: vai ao foreground ANTES de validar; erro → stopStream().
        startForeground(NOTIF_ID, buildNotification("", reconnecting = false))

        if (!AppForeground.isVisible()) {
            // Armadilha §6: câmera/FGS de câmera não inicia com app invisível.
            StreamBus.emit(StreamEvent.Error(StreamErrorCodes.APP_NOT_VISIBLE))
            log("start recusado: app não visível")
            stopStream()
            return
        }
        if (ContextCompat.checkSelfPermission(this, android.Manifest.permission.CAMERA) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            StreamBus.emit(StreamEvent.Error(StreamErrorCodes.CAMERA_PERMISSION_DENIED))
            log("start recusado: permissão de câmera negada")
            stopStream()
            return
        }
        val uri = PairingUri.parse(intent?.getStringExtra(EXTRA_URI).orEmpty()).getOrNull()
        if (uri == null) {
            StreamBus.emit(StreamEvent.Error(StreamErrorCodes.BAD_URI))
            log("start recusado: URI inválida")
            stopStream()
            return
        }
        if (uri.name.orEmpty().isNotEmpty()) currentPc = uri.name.orEmpty()

        val s = StreamSession(
            context = this,
            target = uri,
            store = store,
            scope = scope,
            lifecycleOwner = owner,
            onEvent = ::onSessionEvent,
        )
        session = s
        owner.markActive()
        s.start()
    }

    // ------------------------------ stop ------------------------------

    private fun stopStream() {
        val s = session
        session = null
        s?.stop()
        owner.markIdle()
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    /** Eventos da sessão → bus (plugin/UI) + atualização da notificação. */
    private fun onSessionEvent(ev: StreamEvent) {
        StreamBus.emit(ev)
        when (ev) {
            is StreamEvent.Streaming -> {
                currentPc = ev.pcName
                updateNotification(reconnecting = false)
            }
            StreamEvent.Reconnecting -> updateNotification(reconnecting = true)
            StreamEvent.Idle -> {
                if (session != null) stopStream() // término por erro definitivo (§12)
            }
            else -> Unit
        }
    }

    // ------------------------------ notificação ------------------------------

    private fun updateNotification(reconnecting: Boolean) {
        val nm = getSystemService(NotificationManager::class.java) ?: return
        val quality = session?.qualityLabel ?: DEFAULT_QUALITY
        nm.notify(NOTIF_ID, buildNotification(currentPc, reconnecting, quality))
    }

    private fun buildNotification(pc: String, reconnecting: Boolean, quality: String = DEFAULT_QUALITY): Notification {
        // Texto **só** de strings.xml (F2-T03b): {PC}/{qualidade}/{transporte}.
        // "Wi-Fi" é o mesmo termo em pt-BR e en (Fase 2 só tem Wi-Fi; USB vem depois).
        val text = if (reconnecting) {
            getString(R.string.notif_text_reconnecting)
        } else {
            getString(R.string.notif_text_streaming)
                .replace("{PC}", pc.ifEmpty { getString(R.string.app_name) })
                .replace("{quality}", quality)
                .replace("{transport}", TRANSPORT_WIFI)
        }
        val stopPi = PendingIntent.getService(
            this,
            0,
            Intent(this, StreamingService::class.java).setAction(ACTION_STOP),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(getString(R.string.notif_title_streaming))
            .setContentText(text)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .addAction(0, getString(R.string.notif_action_stop), stopPi)
            .build()
    }

    private fun ensureChannel() {
        if (Build.VERSION.SDK_INT < 26) return
        val nm = getSystemService(NotificationManager::class.java) ?: return
        val ch = NotificationChannel(
            CHANNEL_ID,
            getString(R.string.channel_streaming_name),
            NotificationManager.IMPORTANCE_LOW,
        )
        ch.description = getString(R.string.channel_streaming_desc)
        nm.createNotificationChannel(ch)
    }

    private fun log(msg: String) {
        Log.i(TAG, msg)
    }

    /** Owner sem Activity: o serviço detém o ciclo de vida da sessão. */
    private class ServiceLifecycleOwner : LifecycleOwner {
        private val registry = LifecycleRegistry(this)

        override val lifecycle: Lifecycle
            get() = registry

        fun markActive() {
            registry.currentState = Lifecycle.State.RESUMED
        }

        fun markIdle() {
            registry.currentState = Lifecycle.State.CREATED
        }
    }

    companion object {
        const val TAG = "StreamingService"

        const val ACTION_START = "com.localcamera.app.action.STREAM_START"
        const val ACTION_STOP = "com.localcamera.app.action.STREAM_STOP"
        const val EXTRA_URI = "com.localcamera.app.extra.URI"

        const val CHANNEL_ID = "streaming"
        const val NOTIF_ID = 4865
        const val DEFAULT_QUALITY = "1080p30"
        private const val TRANSPORT_WIFI = "Wi-Fi"

        /**
         * Inicia a sessão (F2-T13 chama de `connect`): [uri] é a string bruta
         * `localcamera://connect?…` (validada no serviço). `ContextCompat`
         * cobre minSdk 24 (`startForegroundService` é API 26+).
         */
        @JvmStatic
        fun startStreaming(context: Context, uri: String) {
            val intent = Intent(context, StreamingService::class.java)
                .setAction(ACTION_START)
                .putExtra(EXTRA_URI, uri)
            ContextCompat.startForegroundService(context, intent)
        }

        /** Para a sessão (F2-T13 chama de `disconnect`; ação "Parar" da notificação). */
        @JvmStatic
        fun stopStreaming(context: Context) {
            context.startService(Intent(context, StreamingService::class.java).setAction(ACTION_STOP))
        }
    }
}
