package com.localcamera.app.encoder

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaCodecList
import android.media.MediaFormat
import android.os.Build
import android.util.Log
import android.view.Surface
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Isolated software-encoder diagnostic; its measurements are not an IT-2.02 acceptance result. */
@RunWith(AndroidJUnit4::class)
class SoftwareCbrDiagnosticTest {
    @Test
    fun software_avc_cbr_pipeline_receives_video() {
        // Diagnóstico opcional: explora codecs de software, fora do requisito
        // de hardware; não substitui o IT-2.02 obrigatório.
        assumeTrue("diagnóstico exige -e runDiagnostics true", InstrumentationRegistry.getArguments()
            .getString("runDiagnostics") == "true")
        check(Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) { "softwareOnly requires API 29+" }
        val cfg = EncoderConfig()
        val candidate = MediaCodecList(MediaCodecList.REGULAR_CODECS).codecInfos.firstOrNull { info ->
            if (!info.isEncoder || !info.isSoftwareOnly || info.supportedTypes.none { it.equals(MIME_AVC, true) }) {
                return@firstOrNull false
            }
            val caps = try { info.getCapabilitiesForType(MIME_AVC) } catch (_: IllegalArgumentException) { null }
            caps != null && caps.encoderCapabilities?.isBitrateModeSupported(
                MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR,
            ) == true && caps.colorFormats.contains(MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface) &&
                caps.isFormatSupported(probeFormat(cfg))
        } ?: throw AssertionError("No REGULAR software AVC encoder supports CBR, surface and default format")

        var encoder: MediaCodec? = null
        var decoder: MediaCodec? = null
        var surface: Surface? = null
        var encStarted = false
        var decStarted = false
        var failure: String? = null
        var fed = 0
        var rendered = 0
        var outputBuffers = 0
        var videoFrames = 0
        var videoBytes = 0L
        var elapsedMs = 0L
        val tag = "SoftwareCbrDiagnostic"

        try {
            val context = InstrumentationRegistry.getInstrumentation().targetContext.applicationContext
            val units = H264Fixture.splitAccessUnits(context.assets.open(ASSET).use { it.readBytes() })
            check(units.size >= FRAME_COUNT) { "fixture has only ${units.size} access units" }

            val args = InstrumentationRegistry.getArguments()
            val softwareDecoder = args.getString("softwareDecoder")?.toBooleanStrictOrNull() ?: false
            if (softwareDecoder) {
                check(Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) { "softwareDecoder requires API 29+" }
            }
            val decoderFormat = MediaFormat.createVideoFormat(MIME_AVC, cfg.width, cfg.height)
            val decInfo = MediaCodecList(MediaCodecList.REGULAR_CODECS).codecInfos.firstOrNull { info ->
                if (info.isEncoder || info.supportedTypes.none { it.equals(MIME_AVC, true) }) return@firstOrNull false
                if (softwareDecoder && !info.isSoftwareOnly) return@firstOrNull false
                val caps = try { info.getCapabilitiesForType(MIME_AVC) } catch (_: IllegalArgumentException) { null }
                caps?.isFormatSupported(decoderFormat) == true
            } ?: throw AssertionError(
                if (softwareDecoder) "No REGULAR software-only AVC decoder supports fixture dimensions"
                else "No AVC decoder supports fixture dimensions",
            )
            Log.i(tag, "scenario=software AVC encoder + ${if (softwareDecoder) "software-only" else "regular"} AVC decoder " +
                "encoder=${candidate.name} decoder=${decInfo.name} softwareDecoder=$softwareDecoder CBR=true bitrate=8000000")

            encoder = MediaCodec.createByCodecName(candidate.name)
            // c2.android.avc.encoder rejeita esta opção; o app já prevê
            // anexar SPS/PPS por conta própria quando o codec não o faz.
            val format = buildFormat(cfg, null, false, true).apply {
                removeKey(MediaFormat.KEY_PREPEND_HEADER_TO_SYNC_FRAMES)
            }
            encoder.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
            surface = encoder.createInputSurface()
            encoder.start()
            encStarted = true

            decoder = MediaCodec.createByCodecName(decInfo.name)
            decoder.configure(decoderFormat, surface, null, 0)
            decoder.start()
            decStarted = true
            Log.i(tag, "encoder=${candidate.name} decoder=${decInfo.name} softwareDecoder=$softwareDecoder CBR=true surface=true")

            val start = System.nanoTime()
            val endNs = start + RUN_MS * 1_000_000
            val encInfo = MediaCodec.BufferInfo()
            val decInfoOut = MediaCodec.BufferInfo()
            var lastFeedNs = start - FRAME_NS
            while (System.nanoTime() < endNs && fed < FRAME_COUNT) {
                val now = System.nanoTime()
                if (fed < FRAME_COUNT && now - lastFeedNs >= FRAME_NS) {
                    val index = decoder.dequeueInputBuffer(DEQUEUE_US)
                    if (index >= 0) {
                        val input = decoder.getInputBuffer(index) ?: error("decoder input buffer unavailable")
                        val unit = units[fed]
                        require(unit.size <= input.capacity()) { "fixture AU ${fed + 1} exceeds decoder buffer" }
                        input.clear(); input.put(unit)
                        decoder.queueInputBuffer(index, 0, unit.size, fed * FRAME_PTS_US, 0)
                        fed++
                        lastFeedNs = now
                    }
                }
                rendered += DecoderSurfaceDrain.drain(decoder, decInfoOut)
                while (true) {
                    val index = encoder.dequeueOutputBuffer(encInfo, 0)
                    if (index == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED || index == MediaCodec.INFO_TRY_AGAIN_LATER) break
                    if (index < 0) break
                    outputBuffers++
                    if (encInfo.flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG == 0 && encInfo.size > 0) {
                        videoBytes += encInfo.size
                        videoFrames++
                    }
                    encoder.releaseOutputBuffer(index, false)
                }
                if (fed < FRAME_COUNT && System.nanoTime() - lastFeedNs < FRAME_NS) Thread.sleep(1)
            }
            elapsedMs = (System.nanoTime() - start) / 1_000_000
            // Bounded tail drain; no frame payloads are retained.
            val tailUntil = System.nanoTime() + TAIL_DRAIN_MS * 1_000_000
            while (System.nanoTime() < tailUntil) {
                rendered += DecoderSurfaceDrain.drain(decoder, decInfoOut)
                val index = encoder.dequeueOutputBuffer(encInfo, DEQUEUE_US)
                if (index >= 0) {
                    outputBuffers++
                    if (encInfo.flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG == 0 && encInfo.size > 0) {
                        videoBytes += encInfo.size
                        videoFrames++
                    }
                    encoder.releaseOutputBuffer(index, false)
                }
            }
        } catch (t: Throwable) {
            failure = "${t.javaClass.simpleName}: ${t.message}"
            Log.e(tag, "diagnostic failed: $failure", t)
        } finally {
            // Stop/release the decoder producer before touching the encoder.
            decoder?.let { codec ->
                try { if (decStarted) codec.stop() } catch (t: Throwable) { Log.w(tag, "decoder stop: ${t.message}") }
                try { codec.release() } catch (t: Throwable) { Log.w(tag, "decoder release: ${t.message}") }
            }
            encoder?.let { codec ->
                try { if (encStarted) codec.stop() } catch (t: Throwable) { Log.w(tag, "encoder stop: ${t.message}") }
                try { codec.release() } catch (t: Throwable) { Log.w(tag, "encoder release: ${t.message}") }
            }
            try { surface?.release() } catch (t: Throwable) { Log.w(tag, "surface release: ${t.message}") }
        }

        val bitrate = if (elapsedMs > 0) videoBytes * 8_000L / elapsedMs else 0L
        Log.i(tag, "encoder=${candidate.name} failure=$failure fed=$fed rendered=$rendered " +
            "outputBuffers=$outputBuffers videoFrames=$videoFrames videoBytes=$videoBytes " +
            "elapsedMs=$elapsedMs bitrate=${bitrate}bps (diagnostic only; not 8Mbps proof)")
        assertTrue("software encoder pipeline failed: $failure", failure == null)
        assertTrue("no decoder frames rendered (fed=$fed rendered=$rendered)", fed > 0 && rendered > 0)
        assertTrue("no encoded video data (buffers=$outputBuffers)", videoBytes > 0 && videoFrames > 0)
    }

    private companion object {
        const val TAG = "SoftwareCbrDiagnostic"
        const val MIME_AVC = "video/avc"
        const val ASSET = "pattern1080.h264"
        const val FRAME_COUNT = 300
        const val FRAME_NS = 1_000_000_000L / 30
        const val FRAME_PTS_US = 1_000_000L / 30
        const val RUN_MS = 10_000L
        const val TAIL_DRAIN_MS = 500L
        const val DEQUEUE_US = 5_000L
    }
}
