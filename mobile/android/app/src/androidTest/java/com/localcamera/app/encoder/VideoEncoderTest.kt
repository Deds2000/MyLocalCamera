package com.localcamera.app.encoder

import android.media.MediaCodec
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
 * O decoder recebe um access unit por buffer, a 30 fps de relógio. Isso mantém
 * o teste no mesmo ritmo do S3, independentemente do tamanho dos buffers que
 * cada decoder oferece.
 */
@RunWith(AndroidJUnit4::class)
class VideoEncoderTest {

    private val frames = CopyOnWriteArrayList<EncodedFrame>()
    private var fixture = ByteArray(0)
    private var fixtureUnits = emptyList<ByteArray>()
    private var encoder: VideoEncoder? = null
    private var decoder: MediaCodec? = null
    private var collector: Thread? = null

    @Test
    fun it201_annexb_keyframes_com_sps_e_pps_intervalo_1s() {
        setUp()
        val runMs = try {
            pump(decoder!!, RUN_MS)
        } finally {
            tearDown()
        }
        Log.i(TAG, "it201 pump concluído (${runMs}ms); iniciando teardown")
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
    fun it202_observa_bitrate_sem_exigir_cbr_fixo() {
        setUp()
        val runMs = try {
            pump(decoder!!, RUN_MS)
        } finally {
            tearDown()
        }
        Log.i(TAG, "it202 pump concluído (${runMs}ms); iniciando teardown")
        val totalBytes = frames.sumOf { it.annexb.size.toLong() }
        val bitrateBps = if (runMs > 0) totalBytes * 8_000L / runMs else 0L
        Log.i(
            TAG,
            "bitrate observado=${bitrateBps}bps (referência inicial 8Mbps; sem limite CBR) " +
                "totalBytes=$totalBytes runMs=$runMs frames=${frames.size}",
        )
        assertTrue(
            "poucos quadros para medir bitrate (${frames.size}, esperado >= $MIN_FRAMES)",
            frames.size >= MIN_FRAMES,
        )
        assertTrue("bitrate não observável (${bitrateBps}bps)", totalBytes > 0 && runMs > 0)
    }

    @Test
    fun it203_requestkeyframe_responde_em_ate_300ms() {
        setUp()
        val enc = encoder!!
        val requestPtsUs = AtomicLong(0)
        val runMs = try {
            pump(decoder!!, RUN_MS_203, MID_MS) { elapsedMs ->
                requestPtsUs.set(System.nanoTime() / 1000)
                enc.requestKeyframe()
                Log.i(TAG, "requestKeyframe enviado em ${elapsedMs}ms")
            }
        } finally {
            tearDown()
        }
        Log.i(TAG, "it203 pump concluído (${runMs}ms); iniciando teardown")
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
        frames.clear()
        fixture = context.assets.open(ASSET_NAME).use { it.readBytes() }
        fixtureUnits = H264Fixture.splitAccessUnits(fixture)
        check(fixtureUnits.size == FIXTURE_FRAME_COUNT) {
            "fixture tem ${fixtureUnits.size} access units; esperado $FIXTURE_FRAME_COUNT"
        }
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
        Log.i(TAG, "teardown: decoder.stop início")
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
        Log.i(TAG, "teardown: decoder liberado")
        decoder = null
        Log.i(TAG, "teardown: encoder.stop início")
        encoder?.stop()
        Log.i(TAG, "teardown: encoder.stop concluído")
        Log.i(TAG, "teardown: collector.join início")
        collector?.join(JOIN_TIMEOUT_MS)
        Log.i(TAG, "teardown: collector.join concluído")
        collector = null
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
        // O Surface não é selecionado pelo KEY_COLOR_FORMAT do decoder: é
        // solicitado pelo argumento `surface` de configure(). Forçar
        // COLOR_FormatSurface aqui faz o OMX do Moto recusar o formato e cair
        // em saída por buffers, que este teste não drena.
        d.configure(fmt, output, null, 0)
        d.start()
        Log.i(TAG, "decoder=${info.name}")
        return d
    }

    /**
     * Alimenta um access unit por buffer, em [FIXTURE_FPS] fps, por [durationMs]
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
        val framesToFeed = minOf(fixtureUnits.size, (durationMs * FIXTURE_FPS / 1_000).toInt())
        val decoderOutputInfo = MediaCodec.BufferInfo()
        Log.i(TAG, "pump iniciado duration=${durationMs}ms accessUnits=$framesToFeed")
        var frameIndex = 0
        var midFired = false
        var tryAgainCount = 0
        var renderedFrames = 0
        while (frameIndex < framesToFeed) {
            val elapsedMs = (System.nanoTime() - t0) / 1_000_000
            if (elapsedMs >= durationMs) break
            if (midAtMs != null && !midFired && elapsedMs >= midAtMs) {
                midAction?.invoke(elapsedMs)
                midFired = true
            }
            if (frameIndex % FIXTURE_FPS.toInt() == 0) {
                Log.i(TAG, "pump: dequeue frame=$frameIndex elapsed=${elapsedMs}ms")
            }
            renderedFrames += DecoderSurfaceDrain.drain(dec, decoderOutputInfo)
            val idx = dec.dequeueInputBuffer(DEQUEUE_TIMEOUT_US)
            if (idx == MediaCodec.INFO_TRY_AGAIN_LATER) {
                renderedFrames += DecoderSurfaceDrain.drain(dec, decoderOutputInfo)
                tryAgainCount++
                if (tryAgainCount % 100 == 0) Log.w(TAG, "pump: dequeueInputBuffer ainda indisponível ($tryAgainCount)")
                Thread.sleep(1)
                continue
            }
            if (idx < 0) {
                Log.e(TAG, "pump: dequeueInputBuffer retornou $idx no frame $frameIndex")
                break
            }
            val buf = dec.getInputBuffer(idx)
            if (buf == null) {
                Log.w(TAG, "getInputBuffer($idx) == null — pump interrompido")
                break
            }
            val unit = fixtureUnits[frameIndex]
            check(unit.size <= buf.capacity()) {
                "access unit ${frameIndex + 1} tem ${unit.size}B; buffer do decoder tem ${buf.capacity()}B"
            }
            buf.clear()
            buf.put(unit)
            val ptsUs = frameIndex * 1_000_000L / FIXTURE_FPS
            dec.queueInputBuffer(idx, 0, unit.size, ptsUs, 0)
            renderedFrames += DecoderSurfaceDrain.drain(dec, decoderOutputInfo)
            if (frameIndex == 0) Log.i(TAG, "pump: primeiro AU ${unit.size}B enviado (buffer=$idx ptsUs=$ptsUs)")
            tryAgainCount = 0
            frameIndex++
            val targetMs = frameIndex * 1_000.0 / FIXTURE_FPS
            val elapsedAfterQueueMs = (System.nanoTime() - t0) / 1_000_000.0
            val waitMs = targetMs - elapsedAfterQueueMs
            if (waitMs > 2) Thread.sleep(waitMs.toLong())
        }
        renderedFrames += DecoderSurfaceDrain.drain(dec, decoderOutputInfo)
        val elapsedMs = (System.nanoTime() - t0) / 1_000_000
        Log.i(
            TAG,
            "pump encerrado elapsed=${elapsedMs}ms accessUnits=$frameIndex/$framesToFeed rendered=$renderedFrames",
        )
        return elapsedMs
    }

    private companion object {
        private const val TAG = "VideoEncoderTest"
        private const val MIME_AVC = "video/avc"
        private const val ASSET_NAME = "pattern1080.h264"

        /** Fixture gerada com 600 access units: 20 s a 30 fps. */
        private const val FIXTURE_FPS = 30L
        private const val FIXTURE_FRAME_COUNT = 600
        private const val RUN_MS = 10_000L
        private const val RUN_MS_203 = 4_000L
        private const val MID_MS = 2_000L
        private const val JOIN_TIMEOUT_MS = 10_000L
        private const val DEQUEUE_TIMEOUT_US = 10_000L

        /** Margem conservadora: ~10 s a 30 fps. */
        private const val MIN_FRAMES = 200
        private const val MIN_KEYFRAMES = 5
    }
}
