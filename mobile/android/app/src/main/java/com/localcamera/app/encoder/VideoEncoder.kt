package com.localcamera.app.encoder

import android.media.MediaCodec
import android.os.Bundle
import android.util.Log
import android.view.Surface
import java.io.IOException
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.receiveAsFlow

/**
 * Encoder H.264 de hardware (cartão F2-T08): 1080p30 CBR 8 Mbps, keyframe
 * a cada 1 s, prioridade tempo real, perfil High com fallback (Main →
 * Baseline), SPS/PPS anexados a **cada** keyframe e `ptsUs` do relógio
 * monotônico injetável ([clock], em ns — o mesmo que o áudio usará na
 * Fase 8; o PC estima o offset na chegada, protocolo §10).
 *
 * Contrato (a `StreamingService` do F2-T12 liga as peças):
 * 1. [configure] — escolhe o encoder (sem nome fixo — `MediaCodecList`,
 *    [scanEncoders]/[pickEncoder]/[configureCodec]), cria a [inputSurface]
 *    **antes** do `configure` (contrato do MediaCodec) e aplica a
 *    configuração com fallback;
 * 2. a câmera se conecta em [inputSurface] (`CameraBackend.start`);
 * 3. [start] — drena a saída em thread própria; [output] entrega os quadros
 *    (1 access unit Annex-B cada) para a `SendQueue`;
 * 4. [requestKeyframe]/[setBitrate] — comandos assíncronos (`setParameters`);
 * 5. [stop] — idempotente; encerra [output].
 *
 * `BUFFER_FLAG_CODEC_CONFIG` = SPS/PPS: **guardado, nunca emitido** (não é
 * vídeo). Se o encoder não pré-preende SPS/PPS no keyframe (armadilha §6),
 * os bytes guardados são anexados antes do IDR (e `flags` bit1 vai junto).
 */
class VideoEncoder(private val clock: () -> Long = System::nanoTime) {

    /** Superfície de entrada para a câmera. Disponível após [configure]. */
    val inputSurface: Surface
        get() {
            val s = surface ?: throw IllegalStateException("configure() antes de usar inputSurface")
            return s
        }

    /** Encoder escolhido (diagnóstico — passo 7 do cartão; nada é nome fixo). */
    @Volatile
    var encoderName: String = ""
        private set

    /** Perfil escolhido pelo fallback (`H264Profile.name`) → STREAM_CONFIG. */
    @Volatile
    var profile: String = "unspecified"
        private set

    /** Quadros descartados pela saída cheia (chegou tarde; para STATS). */
    @Volatile
    var dropped: Int = 0
        private set

    /** Quadros codificados (contador p/ diagnóstico). */
    @Volatile
    var frameCount: Int = 0
        private set

    private val channel = Channel<EncodedFrame>(CHANNEL_CAPACITY)

    /** Quadros H.264 prontos (1 access unit cada); F2-T12 consome. */
    val output: Flow<EncodedFrame> = channel.receiveAsFlow()

    @Volatile private var codec: MediaCodec? = null
    @Volatile private var surface: Surface? = null
    @Volatile private var running = false
    @Volatile private var stopped = false
    private var drainThread: Thread? = null
    private var paramSets: ByteArray? = null
    private var t0Ns = 0L
    @Volatile private var firstKeyframeMs: Int? = null
    private val assembler = AnnexBAssembler()
    private val info = MediaCodec.BufferInfo()
    private val lock = Any()
    private val firstPtsLogged = AtomicBoolean()

    // ------------------------------ configure ------------------------------

    /** Escolhe o encoder e aplica a configuração (ver [EncoderSetup.kt]). */
    fun configure(cfg: EncoderConfig) {
        synchronized(lock) {
            if (codec != null) {
                throw IllegalStateException("configure() já foi chamado (crie um novo VideoEncoder)")
            }
            val (candidates, capsMap) = scanEncoders(cfg)
            val pick = pickEncoder(candidates) ?:
                throw IllegalStateException("nenhum encoder H.264 para ${cfg.width}x${cfg.height}@${cfg.fps}")
            val c = try {
                MediaCodec.createByCodecName(pick.name)
            } catch (e: IOException) {
                throw IllegalStateException("falha ao criar encoder ${pick.name}", e)
            }
            try {
                val s = c.createInputSurface()
                val (profile, lowLatency) = configureCodec(c, s, cfg, profileAttempts(pick.supportedProfiles))
                this.profile = profile?.let { H264Profile.name(it) } ?: "unspecified"
                encoderName = pick.name
                surface = s
                codec = c
                Log.i(
                    LOG_TAG,
                    "encoder=${pick.name} profile=${this.profile} " +
                        "levels=${levelsFor(capsMap[pick.name], profile)} " +
                        "CBR=${cfg.bitrateKbps * 1000}bps " +
                        "${cfg.width}x${cfg.height}@${cfg.fps} kf=${cfg.keyframeIntervalS}s " +
                        "lowLatency=$lowLatency",
                )
            } catch (e: Throwable) {
                c.release()
                throw e
            }
        }
    }

    // ------------------------------ ciclo de vida ------------------------------

    /** Inicia o codec e a thread de drenagem. [inputSurface] já existe desde [configure]. */
    fun start() {
        val c = synchronized(lock) {
            val c = codec ?: throw IllegalStateException("configure() antes de start()")
            if (running) {
                Log.w(LOG_TAG, "start() ignorado: já está rodando")
                return
            }
            running = true
            t0Ns = clock()
            c
        }
        try {
            c.start()
        } catch (t: Throwable) {
            synchronized(lock) { running = false }
            throw t
        }
        drainThread = Thread({ drain(c) }, "lc-encoder-out").apply { start() }
        Log.i(LOG_TAG, "iniciado (encoder=$encoderName)")
    }

    /** Assíncrono (cartão): `PARAMETER_KEY_REQUEST_SYNC_FRAME`. */
    fun requestKeyframe() {
        val c = codec ?: return
        try {
            c.setParameters(Bundle().apply { putString(PARAM_REQUEST_SYNC_FRAME, "1") })
        } catch (t: IllegalStateException) {
            Log.w(LOG_TAG, "requestKeyframe: encoder parado")
        }
    }

    /** Assíncrono (cartão): `PARAMETER_KEY_BIT_RATE` em bps. */
    fun setBitrate(kbps: Int) {
        require(kbps > 0) { "bitrate $kbps kbps deve ser > 0" }
        val c = codec ?: return
        try {
            c.setParameters(Bundle().apply { putInt(PARAM_BIT_RATE, kbps * 1000) })
            Log.i(LOG_TAG, "bitrate pedido: ${kbps}kbps")
        } catch (t: IllegalStateException) {
            Log.w(LOG_TAG, "setBitrate: encoder parado")
        }
    }

    /** Idempotente: encerra o codec, drena o resto e fecha [output]. */
    fun stop() {
        synchronized(lock) {
            if (codec == null && !running) return
            running = false
            stopped = true
        }
        drainThread?.let { t ->
            try {
                t.join(STOP_JOIN_MS)
            } catch (e: InterruptedException) {
                Thread.currentThread().interrupt()
            }
        }
        drainThread = null
        releaseInternal()
        channel.close()
        Log.i(LOG_TAG, "parado (frames=$frameCount, dropped=$dropped, 1ºkf=${firstKeyframeMs}ms)")
    }

    private fun releaseInternal() {
        synchronized(lock) {
            val active = codec
            if (active != null) {
                try {
                    active.stop()
                } catch (t: Throwable) {
                    Log.w(LOG_TAG, "codec.stop: ${t.message}")
                }
                try {
                    active.release()
                } catch (t: Throwable) {
                    Log.w(LOG_TAG, "codec.release: ${t.message}")
                }
                codec = null
            }
            surface?.let { s ->
                try {
                    s.release()
                } catch (t: Throwable) {
                    Log.w(LOG_TAG, "surface.release: ${t.message}")
                }
                surface = null
            }
        }
    }

    // ------------------------------ saída ------------------------------

    private fun drain(c: MediaCodec) {
        try {
            while (!stopped) {
                val idx = c.dequeueOutputBuffer(info, DEQUEUE_TIMEOUT_US)
                when {
                    idx == MediaCodec.INFO_TRY_AGAIN_LATER -> Unit
                    idx == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED -> Log.i(LOG_TAG, "outputFormatChanged")
                    idx < 0 -> {
                        Log.e(LOG_TAG, "erro do codec (code=$idx)")
                        break
                    }
                    else -> {
                        val bytes = copyOutput(c, idx)
                        c.releaseOutputBuffer(idx, false)
                        if (bytes != null) processBuffer(bytes, info.flags, info.presentationTimeUs)
                    }
                }
            }
            if (!stopped) {
                for (unit in assembler.flush()) emit(unit, clock() / NS_PER_US)
            }
        } catch (t: Throwable) {
            // Falha fatal da drenagem: fecha [output] para o consumidor ver o fim.
            Log.e(LOG_TAG, "drain falhou: ${t.javaClass.simpleName}: ${t.message}")
            channel.close()
        }
    }

    /** Copia o buffer de saída (offset..offset+size); `null` se vazio. */
    private fun copyOutput(c: MediaCodec, idx: Int): ByteArray? {
        val n = info.size
        if (n <= 0) return null
        val src = c.getOutputBuffer(idx) ?: return null
        val out = ByteArray(n)
        src.get(info.offset, out) // get absoluto (não move o cursor do buffer)
        return out
    }

    /** Buffer de saída → guardado SPS/PPS ou access unit(s) Anexo-B → [output]. */
    private fun processBuffer(bytes: ByteArray, flags: Int, bufferPtsUs: Long) {
        if (flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG != 0) {
            val first = paramSets == null
            paramSets = bytes
            Log.i(
                LOG_TAG,
                if (first) "SPS/PPS guardados (${bytes.size}B)"
                else "SPS/PPS novos — guardados substituídos (${bytes.size}B)",
            )
            return
        }
        val ptsUs = clock() / NS_PER_US // relógio monotônico (não o bufferPtsUs — logado 1×)
        if (firstPtsLogged.compareAndSet(false, true)) {
            Log.i(LOG_TAG, "ptsUs=$ptsUs bufferPtsUs=$bufferPtsUs (usando o clock)")
        }
        for (unit in assembler.add(bytes)) emit(unit, ptsUs)
    }

    /** SPS/PPS quando faltarem (armadilha §6), flags, e envio sem bloqueio. */
    private fun emit(unit: ByteArray, ptsUs: Long) {
        var payload = unit
        if (H264.isIdr(unit) && !H264.hasParamSets(unit)) {
            val ps = paramSets
            if (ps != null) {
                payload = H264.prependParamSets(ps, unit)
            } else {
                Log.e(LOG_TAG, "keyframe sem SPS/PPS e nada guardado (anômalo)")
            }
        }
        val frame = EncodedFrame(ptsUs, payload, H264.flagsFor(payload))
        frameCount++
        if (frame.isKeyframe && firstKeyframeMs == null) {
            firstKeyframeMs = ((clock() - t0Ns) / 1_000_000).toInt()
        }
        if (!channel.trySend(frame).isSuccess) {
            dropped++
            Log.w(LOG_TAG, "saída cheia — quadro descartado (dropped=$dropped)")
        }
    }

    private companion object {
        const val NS_PER_US = 1000L
        const val DEQUEUE_TIMEOUT_US = 10_000L
        const val STOP_JOIN_MS = 500L
        const val CHANNEL_CAPACITY = 64
        // Parâmetros `MediaCodec.setParameters` ausentes dos stubs públicos
        // (android.jar 34+); valores da fonte AOSP e receita validada no S3:
        const val PARAM_REQUEST_SYNC_FRAME = "sync-frame" // = PARAMETER_KEY_REQUEST_SYNC_FRAME
        const val PARAM_BIT_RATE = "bitrate" // = PARAMETER_KEY_BIT_RATE
    }
}
