package com.localcamera.app.service

import android.content.Context
import android.util.Log
import androidx.lifecycle.LifecycleOwner
import com.localcamera.app.camera.CameraBackend
import com.localcamera.app.camera.CameraConfig
import com.localcamera.app.camera.CameraEvent
import com.localcamera.app.camera.CameraEventCode
import com.localcamera.app.camera.CameraXBackend
import com.localcamera.app.camera.Facing
import com.localcamera.app.encoder.EncoderConfig
import com.localcamera.app.encoder.VideoEncoder
import com.localcamera.app.net.ConnectionEvent
import com.localcamera.app.net.Lane
import com.localcamera.app.net.PrimaryConnection
import com.localcamera.app.net.SendQueue
import com.localcamera.app.net.SystemClock
import com.localcamera.app.protocol.Frame
import com.localcamera.app.protocol.MessageType
import com.localcamera.app.protocol.StreamConfig
import com.localcamera.app.protocol.encodeStreamConfig
import com.localcamera.app.state.StateReducer
import com.localcamera.app.store.PairingStore
import com.localcamera.app.uri.PairingUri
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import java.net.InetSocketAddress
import java.net.Socket
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

/**
 * Sessão de streaming (F2-T12): compõe câmera + encoder + conexão + fila + estado.
 * Ordem (cartão): encoder.configure (cria a [VideoEncoder.inputSurface]) → câmera
 * na surface do encoder → [PrimaryConnection] (frames internos por `ctrlSink` →
 * faixa CTRL da [SendQueue] = **escritor único**, §11) → no `WELCOME`: `STATE` +
 * `STREAM_CONFIG{stream:1, rotationHint, facing}` + task de vídeo. Rotação/`SET`
 * → novo `rotationHint` (AT-2.14); definitivos do §12 → [stop] sem reconectar;
 * rede → backoff (F2-T09) e [SendQueue] trocada (F2-T10). HELLO em [HelloBuilder].
 */
class StreamSession(
    private val context: Context,
    private val target: PairingUri,
    private val store: PairingStore,
    private val scope: CoroutineScope,
    private val lifecycleOwner: LifecycleOwner,
    private val onEvent: (StreamEvent) -> Unit,
    private val cameraFactory: (Context, LifecycleOwner) -> CameraBackend = { c, o -> CameraXBackend(c, o) },
    private val log: (String) -> Unit = { msg -> Log.i(TAG, msg) },
) {

    private val running = AtomicBoolean(false)

    @Volatile private var encoder: VideoEncoder? = null
    @Volatile private var camera: CameraBackend? = null
    @Volatile private var queue: SendQueue? = null
    @Volatile private var connection: PrimaryConnection? = null
    @Volatile private var reducer: StateReducer? = null
    @Volatile private var connJob: Job? = null
    @Volatile private var writerJob: Job? = null
    @Volatile private var videoJob: Job? = null
    @Volatile private var statsJob: Job? = null
    @Volatile private var established = false
    @Volatile private var rotationHint = 0
    @Volatile private var activeFacing = Facing.BACK
    @Volatile private var lastBitrate = 0
    private val bytesSent = AtomicLong(0)

    /** Sessão ativa (entre [start] e [stop]). */
    val isRunning: Boolean get() = running.get()

    /** Rótulo de qualidade p/ a notificação (ex.: "1080p30"). */
    val qualityLabel get() = reducer?.state?.video?.let { "${it.height}p${it.fps}" } ?: DEFAULT_QUALITY

    /** Compõe e inicia tudo (main thread — a `CameraXBackend` exige). Idempotente. */
    fun start() {
        if (!running.compareAndSet(false, true)) {
            log("start ignorado (já ativa)")
            return
        }
        onEvent(StreamEvent.Connecting)

        val enc = VideoEncoder().apply { configure(ENCODER_CONFIG) } // 1080p30, 8 Mbps, kf=1s
        encoder = enc

        val red = StateReducer()
        red.onRotation = { setRotationHint(it) } // SET presentation.rotation
        reducer = red
        activeFacing = Facing.fromCode(red.state.camera.active) ?: Facing.BACK
        lastBitrate = red.state.video.bitrateKbps

        queue = newQueue(enc)

        val conn = PrimaryConnection(
            targets = target.hosts.map { InetSocketAddress.createUnresolved(it, target.port) },
            pcId = target.pc.orEmpty(),
            // §11: SO_SNDBUF/NODELAY **antes** do connect (a factory roda antes dele).
            socketFactory = { Socket().apply { SendQueue.applySocketOptions(this) } },
            clock = SystemClock,
            pairingStore = store,
            onEvent = { ev -> onConnectionEvent(ev) },
            ctrlSink = { frame -> queue?.let { q -> q.enqueue(Lane.CTRL, frame); true } ?: false },
        )
        connection = conn

        val cam = cameraFactory(context, lifecycleOwner)
        cam.onRotationHint { setRotationHint(it) } // rotação do aparelho (AT-2.14)
        cam.onEvent { ev -> onCameraEvent(ev) }
        camera = cam
        cam.start(CameraConfig(activeFacing), enc.inputSurface)
        enc.start()
        connJob = scope.launch { conn.start(HelloBuilder.build(context, target, store)) }
        writerJob = scope.launch(Dispatchers.IO) { writerLoop() }
        videoJob = scope.launch(Dispatchers.Default) { videoTask(enc) }
        statsJob = scope.launch { statsLoop() }
        log("sessão iniciada (hosts=${target.hosts.size}, port=${target.port})")
    }

    /** Encerra (qualquer thread); o I/O bloqueante roda no IO. Idempotente. */
    fun stop() {
        if (!running.compareAndSet(true, false)) return
        log("encerrando sessão")
        onEvent(StreamEvent.Idle)
        established = false
        connJob?.cancel()
        videoJob?.cancel()
        statsJob?.cancel()
        val c = connection
        val e = encoder
        val m = camera
        val q = queue
        connection = null; encoder = null; camera = null; queue = null
        connJob = null; writerJob = null; videoJob = null; statsJob = null
        scope.launch(Dispatchers.IO) {
            c?.close(BYE_REASON_USER)
            q?.stop() // run() volta; o writerLoop sai por running=false
            e?.stop()
            m?.stop()
        }
    }

    /** Escritor único: consome a [SendQueue] do socket ativo (F2-T10). */
    private suspend fun writerLoop() {
        var attached: Socket? = null
        while (running.get()) {
            val socket = connection?.socket
            if (socket == null) { delay(SOCKET_POLL_MS); continue }
            if (socket === attached) continue
            attached = socket
            val q = queue ?: break // stop() em curso
            q.run(socket.getOutputStream()) // volta no stop()/falha de escrita
            attached = null
            delay(SOCKET_POLL_MS)
        }
    }

    /** Vídeo: encoder → faixa VIDEO (1 access unit por quadro, stream 1). Sai no
     * cancel do job ([stop]) ou no fim do canal ([VideoEncoder.stop]). */
    private suspend fun videoTask(enc: VideoEncoder) {
        enc.output.collect { f ->
            bytesSent.addAndGet(f.annexb.size.toLong())
            queue?.enqueue( // null = stop() em curso (a task é cancelada em seguida)
                Lane.VIDEO,
                Frame(MessageType.VIDEO.code, f.flags, STREAM_A, f.ptsUs, f.annexb),
            )
        }
    }

    /** Stats agregados p/ a UI, no máx. a cada 500 ms (2 Hz — cartão). */
    private suspend fun statsLoop() {
        var lastFrames = 0
        var lastBytes = 0L
        var lastT = System.currentTimeMillis()
        while (running.get()) {
            delay(STATS_INTERVAL_MS)
            val enc = encoder ?: continue
            val now = System.currentTimeMillis()
            val dt = (now - lastT).coerceAtLeast(1L)
            val frames = enc.frameCount
            val bytes = bytesSent.get()
            // rtt: ainda não medido pela PrimaryConnection (F2-T09) → 0.
            onEvent(StreamEvent.Stats(((frames - lastFrames) * 1000L / dt).toInt(),
                ((bytes - lastBytes) * 8L / dt).toInt(), queue?.queueAgeMs() ?: 0L, 0L))
            lastFrames = frames
            lastBytes = bytes
            lastT = now
        }
    }

    private fun onConnectionEvent(ev: ConnectionEvent) {
        when (ev) {
            is ConnectionEvent.Pending -> onEvent(StreamEvent.Pending(ev.timeoutS))
            is ConnectionEvent.Welcome -> {
                established = true
                log("WELCOME: pc=${ev.pc.name}")
                onEvent(StreamEvent.Streaming(ev.pc.name))
                sendState()
                sendStreamConfig()
            }
            is ConnectionEvent.Denied -> {
                onEvent(StreamEvent.Error(StreamErrorCodes.denied(ev.reason)))
                if (ev.reason in PrimaryConnection.TERMINAL_REASONS) stop() // §12
            }
            ConnectionEvent.Lost -> {
                swapQueue()
                onEvent(StreamEvent.Reconnecting)
            }
            ConnectionEvent.KeyframeRequested -> encoder?.requestKeyframe()
            is ConnectionEvent.Bye -> {
                onEvent(StreamEvent.Error(StreamErrorCodes.bye(ev.reason)))
                stop() // §12: BYE é definitivo
            }
            is ConnectionEvent.Set -> onSet(ev.json)
        }
    }

    private fun onCameraEvent(ev: CameraEvent) {
        when (ev) {
            CameraEvent.Started -> log("câmera iniciada")
            is CameraEvent.Error -> {
                log("erro de câmera: ${ev.code}")
                onEvent(StreamEvent.Error(ev.code))
                if (ev.code in FATAL_CAMERA_CODES) stop() // recuperáveis seguem (F2-T14)
            }
            is CameraEvent.Rotated -> setRotationHint(ev.hint)
        }
    }

    private fun onSet(json: String) {
        val red = reducer ?: return
        val st = red.applySet(json)
        onEvent(StreamEvent.StateChanged(red.stateJson()))
        if (st.video.bitrateKbps != lastBitrate) {
            lastBitrate = st.video.bitrateKbps
            encoder?.setBitrate(lastBitrate)
        }
        val facing = Facing.fromCode(st.camera.active) ?: return
        if (facing != activeFacing) {
            activeFacing = facing
            camera?.switchTo(facing)
            sendStreamConfig()
        }
    }

    /** Rotação do aparelho / SET → novo `rotationHint` no STREAM_CONFIG (AT-2.14). */
    private fun setRotationHint(hint: Int) {
        if (rotationHint == hint) return
        rotationHint = hint
        if (established && running.get()) sendStreamConfig()
    }

    private fun sendState() {
        val json = reducer?.stateJson() ?: return
        queue?.enqueue(Lane.CTRL, Frame(MessageType.STATE.code, 0, 0, 0, json.toByteArray(Charsets.UTF_8)))
        onEvent(StreamEvent.StateChanged(json))
    }

    private fun sendStreamConfig() {
        val s = reducer?.state ?: return
        val sc = StreamConfig(STREAM_A, s.video.width, s.video.height, s.video.fps, rotationHint, s.camera.active)
        queue?.enqueue(Lane.CTRL, Frame(MessageType.STREAM_CONFIG.code, 0, 0, 0, encodeStreamConfig(sc)))
        log("STREAM_CONFIG ${s.video.width}x${s.video.height}@${s.video.fps} rot=$rotationHint face=${s.camera.active}")
    }

    private fun newQueue(enc: VideoEncoder): SendQueue = SendQueue().also { q ->
        q.onDropStale = { enc.requestKeyframe() } // descarte de velhos → keyframe
        q.onWriteError = { log("falha de escrita"); if (running.get()) swapQueue() }
    }

    /** A [SendQueue] é de uso único após falha de escrita (F2-T10): troca na reconexão. */
    @Synchronized
    private fun swapQueue() {
        val old = queue ?: return
        val enc = encoder ?: return
        queue = newQueue(enc)
        old.stop()
    }

    private companion object {
        const val TAG = "StreamSession"
        const val STREAM_A = 1 // modo single: câmera ativa = stream 1 (CHEATSHEET)
        const val BYE_REASON_USER = "user" // o PC trata o BYE genericamente
        const val DEFAULT_QUALITY = "1080p30"
        const val STATS_INTERVAL_MS = 500L
        const val SOCKET_POLL_MS = 100L
        val ENCODER_CONFIG = EncoderConfig() // 1080p30 CBR 8 Mbps kf=1s (CHEATSHEET)
        val FATAL_CAMERA_CODES = setOf(CameraEventCode.CAMERA_IN_USE, CameraEventCode.MAX_CAMERAS_IN_USE,
            CameraEventCode.CAMERA_FATAL, CameraEventCode.CAMERA_REMOVED, CameraEventCode.CAMERA_DISABLED,
            CameraEventCode.START_FAILED, CameraEventCode.CAMERA_UNAVAILABLE, CameraEventCode.CAMERA_CLOSED)
    }
}
