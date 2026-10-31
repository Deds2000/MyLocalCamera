package com.localcamera.app.encoder

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaCodecList
import android.media.MediaFormat
import android.util.Log
import android.view.Surface
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicLong

/**
 * IT-2.01/02/03 (cartão F2-T08, passo 5 — parte H): pipeline completo com
 * a fixture `pattern1080.h264` (1080p30, 20 s) → decoder H.264 →
 * [VideoEncoder.inputSurface] → encoder H.264 → [VideoEncoder.output].
 *
 * O decoder é alimentado em **taxa de bytes constante** (a fixture de 20 s
 * em 20 s = 30 fps em tempo real), para que o encoder veja entrada em tempo
 * real e o intervalo de keyframe (KEY_I_FRAME_INTERVAL=1 s) saia em
 * segundos de relógio — sem pacing o decoder roda em excesso e o intervalo
 * quebra (aprendizado do S3).
 */
@RunWith(AndroidJUnit4::class)
class VideoEncoderTest {

    private val frames = CopyOnWriteArrayList<EncodedFrame>()
    private var fixture = ByteArray(0)
    private var encoder: VideoEncoder? = null
    private var decoder: MediaCodec? = null
    private var collector: Thread? = null

    @Test
    fun it201_annexb_keyframes_com_sps_e_pps_intervalo_1s() {
        setUp()
        val runMs = pump(decoder!!, RUN_MS)
        tearDown()
        Log.i(TAG, "frames=${frames.size} runMs=$runMs encoder=${encoder?.encoderName} " +
            "profile=${encoder?.profile} dropped=${encoder?.dropped}")

        assertTrue(
            "poucos quadros em ${runMs}ms (frames=${frames.size}, esperado >= $MIN_FRAMES)",
            frames.size >= MIN_FRAMES,
        )
        frames.forEach { f ->
            assertTrue(
                "quadro sem start code 00 00 01 (annexb=${f.annexb.size}B)",
                f.annexb.size >= 3 && f.annexb[0] == 0.toByte() && f.annexb[1] == 0.toByte() && f.annexb[2] == 1.toByte(),
            )
        }
        val keyframes = frames.filter { it.isKeyframe }
        assertTrue(
            "poucos keyframes (${keyframes.size} em ${frames.size}, esperado >= $MIN_KEYFRAMES)",
            keyframes.size >= MIN_KEYFRAMES,
        )
        keyframes.forEach { k ->
            assertTrue("keyframe sem SPS+PPS no conteudo: $k", H264.hasParamSets(k.annexb))
            assertTrue("keyframe sem flag SPS/PPS (bit1): $k", k.hasParamSets)
        }
        for (i in 1 until keyframes.size) {
            val dtMs = (keyframes[i].ptsUs - keyframes[i - 1].ptsUs) / 1000
            assertTrue(
                "intervalo de keyframe fora de 1s±100ms: ${dtMs}ms (frame $i)",
                dtMs in 900..1100,
            )
        }
    }

    @Test
    fun it202_cbr_8mbps_mais_ou_menos_15_por_cento() {
        setUp()
        val runMs = pump(decoder!!, RUN_MS)
        tearDown()
        val totalBytes = frames.sumOf { it.annexb.size.toLong() }
        val bitrateBps = if (runMs > 0) totalBytes * 8_000_000L / runMs else 0L
        Log.i(TAG, "totalBytes=$totalBytes runMs=$runMs bitrate=${bitrateBps}bps frames=${frames.size}")
        assertTrue(
            "bitrate ${bitrateBps}bps fora de 8Mbps±15% (bytes=$totalBytes em ${runMs}ms)",
            bitrateBps in (TARGET_BPS * 85 / 100)..(TARGET_BPS * 115 / 100),
        )
    }

    @Test
    fun it203_requestkeyframe_responde_em_ate_300ms() {
        setUp()
        val enc = encoder!!
        val requestPtsUs = AtomicLong(0)
        val runMs = pump(decoder!!, RUN_MS_203, MID_MS) { elapsedMs ->
            requestPtsUs.set(System.nanoTime() / 1000)
            enc.requestKeyframe()
            Log.i(TAG, "requestKeyframe enviado em ${elapsedMs}ms")
        }
        tearDown()
        Log.i(TAG, "frames=${frames.size} runMs=$runMs")
        val keyframes = frames.filter { it.isKeyframe }
        assertTrue("nenhum keyframe no stream (${frames.size} frames)", keyframes.isNotEmpty())
        val next = keyframes.firstOrNull { it.ptsUs >= requestPtsUs.get() }
            ?: throw AssertionError("nenhum keyframe apos o requestKeyframe")
        val dtMs = (next.ptsUs - requestPtsUs.get()) / 1000
        Log.i(TAG, "keyframe apos o request em ${dtMs}ms")
        assertTrue("requestKeyframe respondeu em ${dtMs}ms (limite 300ms)", dtMs <= 300)
    }

    // ------------------------------ montagem ------------------------------

    /** Fixture + encoder configurado + decoder apontando para a inputSurface. */
    private fun setUp() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext.applicationContext
        fixture = context.assets.open(ASSET_NAME).use { it.readBytes() }
        encoder = VideoEncoder().also { it.configure(EncoderConfig()) }
        Log.i(TAG, "encoder=${encoder?.encoderName} profile=${encoder?.profile}")
        decoder = createDecoder(encoder!!.inputSurface)
        val enc = encoder!!
        collector = Thread({
            runBlocking { enc.output.collect { frames.add(it) } }
        }, "lc-test-collector").apply { start() }
        enc.start()
    }

    /**
     * Para o **produtor primeiro** (o decoder, que alimenta a inputSurface) e
     * só depois o encoder: no SM-G781B, `MediaCodec.stop()` do VENC bloqueia
     * para sempre no binder enquanto a superfície ainda recebe frames (wedge
     * observado no IT-2.02: >13 min, threads `lc-*` vivas). `encoder?.stop()`
     * fecha [VideoEncoder.output] → o collector termina.
     */
    private fun tearDown() {
        decoder?.let { d ->
            try {
                d.stop()
            } catch (t: Throwable) {
                Log.w(TAG, "decoder.stop: ${t.message}")
            }
            try {
                d.release()
            } catch (t: Throwable) {
                Log.w(TAG, "decoder.release: ${t.message}")
            }
        }
        decoder = null
        encoder?.stop()
        collector?.join(JOIN_TIMEOUT_MS)
        collector = null
        frames.clear()
    }

    /** Encoder H.264 (decoding) com saída em [output]; nome nada é fixo. */
    private fun createDecoder(output: Surface): MediaCodec {
        val mime = MIME_AVC
        val probe = MediaFormat.createVideoFormat(mime, 1920, 1080).apply {
            setInteger(MediaFormat.KEY_BIT_RATE, 100_000)
            setInteger(MediaFormat.KEY_FRAME_RATE, 30)
        }
        // `MediaCodecInfo.isDecoder` nao esta no stub 36 (existe em runtime):
        // decoder = codec que suporta o mime e nao e encoder.
        val info = MediaCodecList(MediaCodecList.REGULAR_CODECS).codecInfos
            .firstOrNull { c ->
                if (c.isEncoder) return@firstOrNull false
                // `getCapabilitiesForType` LANÇA (IllegalArgumentException) para
                // codec sem o mime — envelopa (não devolve null).
                val caps = try { c.getCapabilitiesForType(mime) } catch (e: IllegalArgumentException) { null }
                caps != null && caps.isFormatSupported(probe)
            }
            ?: throw IllegalStateException("nenhum decoder H.264 disponivel")
        val d = MediaCodec.createByCodecName(info.name)
        val fmt = MediaFormat.createVideoFormat(mime, 1920, 1080)
        fmt.setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
        d.configure(fmt, output, null, 0)
        d.start()
        Log.i(TAG, "decoder=${info.name}")
        return d
    }

    /**
     * Alimenta o decoder com a fixture em taxa de bytes constante
     * (fixture de [FIXTURE_TOTAL_MS] em [FIXTURE_TOTAL_MS]) por [durationMs]
     * de relógio; devolve o tempo real medido (ms). [midAction] roda uma vez
     * em [midAtMs] (null = nunca).
     */
    private fun pump(
        dec: MediaCodec,
        durationMs: Long,
        midAtMs: Long? = null,
        midAction: ((Long) -> Unit)? = null,
    ): Long {
        val t0 = System.nanoTime()
        var pos = 0
        var midFired = false
        while (pos < fixture.size) {
            val elapsedMs = (System.nanoTime() - t0) / 1_000_000
            if (elapsedMs >= durationMs) break
            if (midAtMs != null && !midFired && elapsedMs >= midAtMs) {
                midAction?.invoke(elapsedMs)
                midFired = true
            }
            val idx = dec.dequeueInputBuffer(DEQUEUE_TIMEOUT_US)
            if (idx == MediaCodec.INFO_TRY_AGAIN_LATER) {
                Thread.sleep(1)
                continue
            }
            if (idx < 0) break
            val buf = dec.getInputBuffer(idx)
            if (buf == null) {
                // Caminho anormal (so em codecs sem DDK buffer); o stub 36 nem
                // tem releaseInputBuffer — interrompe o pump e o tearDown limpa.
                Log.w(TAG, "getInputBuffer($idx) == null — pump interrompido")
                break
            }
            val n = minOf(buf.remaining(), fixture.size - pos)
            if (n <= 0) {
                dec.queueInputBuffer(idx, 0, 0, System.nanoTime() / 1000, MediaCodec.BUFFER_FLAG_END_OF_STREAM)
                continue
            }
            buf.clear()
            buf.put(fixture, pos, n)
            pos += n
            dec.queueInputBuffer(idx, 0, n, System.nanoTime() / 1000, 0)
            // Pacing: instante-alvo = pos * total / tamanho (bytes constantes/s).
            val targetMs = (pos * 1000.0 * FIXTURE_TOTAL_MS) / fixture.size
            val waitMs = targetMs - (System.nanoTime() - t0) / 1_000_000.0
            if (waitMs > 2) Thread.sleep(waitMs.toLong())
        }
        return (System.nanoTime() - t0) / 1_000_000
    }

    private companion object {
        private const val TAG = "VideoEncoderTest"
        private const val MIME_AVC = "video/avc"
        private const val ASSET_NAME = "pattern1080.h264"

        /** A fixture tem 20 s; o teste de 10 s consome a metade em 30 fps reais. */
        private const val FIXTURE_TOTAL_MS = 20_000L
        private const val RUN_MS = 10_000L
        private const val RUN_MS_203 = 4_000L
        private const val MID_MS = 2_000L
        private const val JOIN_TIMEOUT_MS = 10_000L
        private const val DEQUEUE_TIMEOUT_US = 10_000L

        /** Margem conservadora: ~10 s a 30 fps. */
        private const val MIN_FRAMES = 200
        private const val MIN_KEYFRAMES = 5
        private const val TARGET_BPS = 8_000_000L
    }
}
