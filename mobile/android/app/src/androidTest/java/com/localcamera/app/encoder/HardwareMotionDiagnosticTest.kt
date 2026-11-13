package com.localcamera.app.encoder

import android.media.MediaCodec
import android.media.MediaCodecList
import android.media.MediaFormat
import android.util.Log
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.flow.collect
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.concurrent.atomic.AtomicLong

/** Motion-content hardware diagnostic; measurements do not establish IT-2.02 acceptance. */
@RunWith(AndroidJUnit4::class)
class HardwareMotionDiagnosticTest {
    @Test
    fun hardware_encoder_with_motion_fixture_emits_video() {
        val tag = "HardwareMotionDiag"
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        // Diagnóstico opcional: depende de fixture externa temporária, que não
        // acompanha a suíte instrumentada nem substitui o IT-2.02 obrigatório.
        assumeTrue("diagnóstico exige -e runDiagnostics true", InstrumentationRegistry.getArguments()
            .getString("runDiagnostics") == "true")
        val externalFiles = instrumentation.targetContext.getExternalFilesDir(null)
            ?: error("App external files directory unavailable")
        val fixture = File(externalFiles, FIXTURE_NAME)
        check(fixture.isFile) { "Diagnostic fixture missing: ${fixture.name}" }
        val units = H264Fixture.splitAccessUnits(fixture.readBytes())
        check(units.size >= FRAME_COUNT) { "Fixture has only ${units.size} access units" }

        val cfg = EncoderConfig()
        val decoderInfo = MediaCodecList(MediaCodecList.REGULAR_CODECS).codecInfos.firstOrNull { info ->
            if (info.isEncoder || info.supportedTypes.none { it.equals(MIME_AVC, ignoreCase = true) }) {
                return@firstOrNull false
            }
            val caps = try { info.getCapabilitiesForType(MIME_AVC) } catch (_: IllegalArgumentException) { null }
            caps?.isFormatSupported(MediaFormat.createVideoFormat(MIME_AVC, cfg.width, cfg.height)) == true
        } ?: error("No AVC decoder supports ${cfg.width}x${cfg.height}")

        var encoder: VideoEncoder? = null
        var decoder: MediaCodec? = null
        var decoderStarted = false
        var collector: Thread? = null
        val byteCount = AtomicLong()
        val frameCount = AtomicLong()
        var submitted = 0
        var rendered = 0
        var durationMs = 0L
        try {
            encoder = VideoEncoder().also { it.configure(cfg) }
            decoder = MediaCodec.createByCodecName(decoderInfo.name)
            decoder.configure(
                MediaFormat.createVideoFormat(MIME_AVC, cfg.width, cfg.height),
                encoder.inputSurface,
                null,
                0,
            )
            decoder.start()
            decoderStarted = true

            val activeEncoder = encoder
            collector = Thread({
                runBlocking {
                    activeEncoder.output.collect { frame ->
                        frameCount.incrementAndGet()
                        byteCount.addAndGet(frame.annexb.size.toLong())
                    }
                }
            }, "lc-motion-diagnostic-collector").apply { start() }
            activeEncoder.start()

            val startNs = System.nanoTime()
            val deadlineNs = startNs + RUN_MS * 1_000_000L
            val outputInfo = MediaCodec.BufferInfo()
            while (submitted < FRAME_COUNT && System.nanoTime() < deadlineNs) {
                val targetNs = startNs + submitted * FRAME_NS
                if (System.nanoTime() < targetNs) {
                    Thread.sleep(1)
                    continue
                }
                val inputIndex = decoder.dequeueInputBuffer(DEQUEUE_US)
                if (inputIndex >= 0) {
                    val input = decoder.getInputBuffer(inputIndex) ?: error("Decoder input unavailable")
                    val unit = units[submitted]
                    require(unit.size <= input.capacity()) { "AU ${submitted + 1} exceeds decoder buffer" }
                    input.clear()
                    input.put(unit)
                    decoder.queueInputBuffer(inputIndex, 0, unit.size, submitted * 1_000_000L / FPS, 0)
                    submitted++
                }
                rendered += DecoderSurfaceDrain.drain(decoder, outputInfo)
            }
            durationMs = ((System.nanoTime() - startNs) / 1_000_000L).coerceAtLeast(1)

            // Bounded drain for the final queued decoder/encoder outputs.
            val tailDeadline = System.nanoTime() + TAIL_DRAIN_MS * 1_000_000L
            while (System.nanoTime() < tailDeadline) {
                rendered += DecoderSurfaceDrain.drain(decoder, outputInfo)
                Thread.sleep(2)
            }
        } finally {
            // Stop the surface producer before the production encoder.
            decoder?.let { codec ->
                try { if (decoderStarted) codec.stop() } catch (t: Throwable) { Log.w(tag, "decoder stop: ${t.message}") }
                try { codec.release() } catch (t: Throwable) { Log.w(tag, "decoder release: ${t.message}") }
            }
            try { encoder?.stop() } catch (t: Throwable) { Log.w(tag, "encoder stop: ${t.message}") }
            collector?.let {
                try { it.join(COLLECTOR_JOIN_MS) } catch (e: InterruptedException) { Thread.currentThread().interrupt() }
                if (it.isAlive) Log.w(tag, "collector still alive after bounded join")
            }
        }

        val bitrate = byteCount.get() * 8_000L / durationMs.coerceAtLeast(1)
        Log.i(tag, "encoder=${encoder?.encoderName} bytes=${byteCount.get()} frames=${frameCount.get()} " +
            "durationMs=$durationMs bitrate=${bitrate}bps submitted=$submitted rendered=$rendered " +
            "dropped=${encoder?.dropped} (diagnostic only; not IT-2.02 acceptance)")
        assertTrue("No encoded video frames", frameCount.get() > 0)
        assertTrue("No encoded video bytes", byteCount.get() > 0)
    }

    private companion object {
        const val MIME_AVC = "video/avc"
        const val FIXTURE_NAME = "lc-diagnostic-hd30-20261102.h264"
        const val FRAME_COUNT = 300
        const val FPS = 30L
        const val FRAME_NS = 1_000_000_000L / FPS
        const val RUN_MS = 10_000L
        const val TAIL_DRAIN_MS = 500L
        const val DEQUEUE_US = 5_000L
        const val COLLECTOR_JOIN_MS = 2_000L
    }
}
