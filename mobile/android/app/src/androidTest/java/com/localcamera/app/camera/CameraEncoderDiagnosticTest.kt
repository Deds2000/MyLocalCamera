package com.localcamera.app.camera

import android.Manifest
import android.util.Log
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import com.localcamera.app.encoder.EncoderConfig
import com.localcamera.app.encoder.VideoEncoder
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.flow.collect
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong

/** Diagnóstico local da cadeia CameraX → superfície do encoder → H.264. */
@RunWith(AndroidJUnit4::class)
class CameraEncoderDiagnosticTest {

    @get:Rule
    val grantCameraPermission = GrantPermissionRule.grant(Manifest.permission.CAMERA)

    @Test
    fun camera_frames_reach_encoder() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        // Opcional: abre câmera física por ~10 s; não substitui os testes do cartão.
        assumeTrue("passe -e runDiagnostics true para habilitar diagnóstico de câmera", args.getString("runDiagnostics") == "true")

        val width = args.getString("width")?.toIntOrNull() ?: DEFAULT_WIDTH
        val height = args.getString("height")?.toIntOrNull() ?: DEFAULT_HEIGHT
        require(width > 0 && height > 0) { "width/height devem ser positivos" }

        val context = instrumentation.targetContext.applicationContext
        val owner = TestLifecycleOwner()
        var ownerStarted = false
        var camera: CameraXBackend? = null
        var encoder: VideoEncoder? = null
        var collector: Thread? = null
        val events = CopyOnWriteArrayList<CameraEvent>()
        val started = CountDownLatch(1)
        val outputFrames = AtomicLong(0)
        val outputBytes = AtomicLong(0)
        val negativeLatencySamples = AtomicLong(0)
        val latencySamplesUs = CopyOnWriteArrayList<Long>()
        val measureLatency = args.getString(ARG_MEASURE_LATENCY) == "true"

        try {
            instrumentation.runOnMainSync {
                owner.start()
                ownerStarted = true
            }

            val enc = VideoEncoder().also {
                it.configure(EncoderConfig(width = width, height = height, fps = TARGET_FPS))
            }
            encoder = enc

            collector = Thread({
                runBlocking {
                    enc.output.collect { frame ->
                        outputFrames.incrementAndGet()
                        outputBytes.addAndGet(frame.annexb.size.toLong())
                        if (measureLatency) {
                            frame.sourcePtsUs?.let { sourcePtsUs ->
                                val latencyUs = frame.ptsUs - sourcePtsUs
                                if (latencyUs >= 0) latencySamplesUs.add(latencyUs)
                                else negativeLatencySamples.incrementAndGet()
                            }
                        }
                    }
                }
            }, "lc-camera-encoder-diagnostic-collector").apply { start() }

            val backend = CameraXBackend(context, owner)
            camera = backend
            backend.onEvent { event ->
                events.add(event)
                when (event) {
                    CameraEvent.Started -> started.countDown()
                    is CameraEvent.Error -> Log.w(TAG, "camera error code=${event.code}")
                    is CameraEvent.Rotated -> Unit
                }
            }

            instrumentation.runOnMainSync {
                backend.start(
                    CameraConfig(Facing.BACK, width = width, height = height, fps = TARGET_FPS),
                    enc.inputSurface,
                )
            }
            enc.start()

            if (!started.await(START_TIMEOUT_S, TimeUnit.SECONDS)) {
                throw IllegalStateException("CameraX não emitiu Started em ${START_TIMEOUT_S}s")
            }

            val firstFrameCount = outputFrames.get()
            val firstByteCount = outputBytes.get()
            val firstLatencyCount = latencySamplesUs.size
            val firstNegativeLatencyCount = negativeLatencySamples.get()
            val startedAtNs = System.nanoTime()
            Thread.sleep(MEASURE_MS)
            val elapsedNs = System.nanoTime() - startedAtNs
            val count = outputFrames.get() - firstFrameCount
            val bytes = outputBytes.get() - firstByteCount
            val measuredLatencies = latencySamplesUs.drop(firstLatencyCount).sorted()
            val negativeLatencyCount = negativeLatencySamples.get() - firstNegativeLatencyCount
            val p95LatencyUs = measuredLatencies.getOrNull(
                ((measuredLatencies.size * 95 + 99) / 100 - 1).coerceAtLeast(0),
            )
            val measuredFps = if (elapsedNs > 0) count * 1_000_000_000.0 / elapsedNs else 0.0
            val errors = events.filterIsInstance<CameraEvent.Error>()
            Log.i(
                TAG,
                "requested=${width}x${height}@$TARGET_FPS measuredFps=${"%.2f".format(measuredFps)} " +
                    "frames=$count bytes=$bytes dropped=${enc.dropped} errors=${errors.map { it.code }} " +
                    "elapsedMs=${elapsedNs / 1_000_000} " +
                    "latencySamples=${measuredLatencies.size} negativeLatency=$negativeLatencyCount " +
                    "p95CameraToEncodedMs=${p95LatencyUs?.div(1000.0) ?: "unmeasured"}",
            )

            assertTrue("nenhum quadro H.264 entregue pelo encoder", count > 0)
            assertTrue("CameraX reportou erro: ${errors.map { it.code }}", errors.isEmpty())
            if (measureLatency) {
                assertTrue("poucas amostras de timestamp de origem (${measuredLatencies.size})", measuredLatencies.size >= 100)
                assertTrue("relógios de captura/encoder não estão correlacionados ($negativeLatencyCount negativos)", negativeLatencyCount == 0L)
                assertTrue("p95 câmera→quadro codificado foi ${p95LatencyUs?.div(1000.0)} ms", (p95LatencyUs ?: Long.MAX_VALUE) <= MAX_P95_LATENCY_US)
            }
        } finally {
            // Feche primeiro o produtor de frames, na main thread, para evitar
            // manter a inputSurface ativa durante VideoEncoder.stop().
            instrumentation.runOnMainSync {
                try {
                    camera?.stop()
                } catch (t: Throwable) {
                    Log.w(TAG, "camera stop failed: ${t.javaClass.simpleName}")
                } finally {
                    if (ownerStarted) owner.finish()
                }
            }
            try {
                encoder?.stop()
            } catch (t: Throwable) {
                Log.w(TAG, "encoder stop failed: ${t.javaClass.simpleName}")
            }
            collector?.let { thread ->
                try {
                    thread.join(COLLECTOR_JOIN_MS)
                    if (thread.isAlive) {
                        Log.w(TAG, "collector join timed out")
                        thread.interrupt()
                    }
                } catch (t: InterruptedException) {
                    Thread.currentThread().interrupt()
                    Log.w(TAG, "collector join interrupted")
                }
            }
        }
    }

    private class TestLifecycleOwner : LifecycleOwner {
        private val registry = LifecycleRegistry(this)

        override val lifecycle: Lifecycle
            get() = registry

        fun start() {
            registry.currentState = Lifecycle.State.RESUMED
        }

        fun finish() {
            registry.currentState = Lifecycle.State.DESTROYED
        }
    }

    private companion object {
        const val TAG = "CameraEncoderDiagnostic"
        const val DEFAULT_WIDTH = 1920
        const val DEFAULT_HEIGHT = 1080
        const val TARGET_FPS = 30
        const val START_TIMEOUT_S = 20L
        const val MEASURE_MS = 10_000L
        const val COLLECTOR_JOIN_MS = 5_000L
        const val ARG_MEASURE_LATENCY = "measureLatency"
        const val MAX_P95_LATENCY_US = 80_000L
    }
}
