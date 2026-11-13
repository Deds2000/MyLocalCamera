package com.localcamera.app.camera

import android.Manifest
import android.content.Context
import android.graphics.ImageFormat
import android.graphics.SurfaceTexture
import android.media.ImageReader
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.util.Size
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

/**
 * Smoke (cartão F2-T07, passo 4): start por 10 s com superfície dummy
 * (`ImageReader`) → eventos `started`, sem erro, quadros chegando (contados
 * no callback de imagem). 1080p30 validado no S3 (29,7 fps, jitter <1).
 */
@RunWith(AndroidJUnit4::class)
class CameraBackendSmokeTest {

    @get:Rule
    val grantCameraPermission = GrantPermissionRule.grant(Manifest.permission.CAMERA)

    @Test
    fun start_10s_started_frames_no_error() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext.applicationContext
        val size = supportedBackPreviewSize(context)
        val owner = TestLifecycleOwner()
        instrumentation.runOnMainSync { owner.start() }
        val backend = CameraXBackend(context, owner)

        val events = CopyOnWriteArrayList<CameraEvent>()
        val started = CountDownLatch(1)
        val hints = AtomicInteger(0)
        backend.onEvent { ev ->
            Log.i(TAG, "evento: $ev")
            events.add(ev)
            if (ev is CameraEvent.Started) started.countDown()
        }
        backend.onRotationHint { hints.incrementAndGet() }

        // Superfície dummy: o ImageReader recebe o que iria para o encoder.
        val reader = ImageReader.newInstance(size.width, size.height, ImageFormat.PRIVATE, 3)
        val frames = AtomicInteger(0)
        reader.setOnImageAvailableListener(
            {
                frames.incrementAndGet()
                try {
                    reader.acquireLatestImage()?.close()
                } catch (t: Throwable) {
                    // fim do teste: o reader é fechado abaixo
                }
            },
            Handler(Looper.getMainLooper()),
        )

        try {
            instrumentation.runOnMainSync {
                backend.start(CameraConfig(Facing.BACK, width = size.width, height = size.height), reader.surface)
            }
            assertTrue("sem evento started em 30 s (eventos: $events)", started.await(30, TimeUnit.SECONDS))
            Thread.sleep(RUN_MS)
        } finally {
            instrumentation.runOnMainSync {
                backend.stop()
                owner.finish()
            }
            reader.close()
        }

        val errors = events.filterIsInstance<CameraEvent.Error>()
        Log.i(TAG, "frames=${frames.get()} hints=${hints.get()} eventos=$events")
        assertTrue("erros durante o smoke: $errors", errors.isEmpty())
        assertTrue("nenhum hint de rotação emitido", hints.get() > 0)
        assertTrue(
            "quadros não chegando (frames=${frames.get()}, esperado >= $MIN_FRAMES)",
            frames.get() >= MIN_FRAMES,
        )
    }

    /**
     * Mesmo modo da sessão de produção: `CapabilitiesProbe.sessionModes` +
     * `SessionModePicker` (primeira câmera back, melhor 16:9 ≤ 1080p). Sem
     * modo verificado, falha o teste — não inventa um tamanho (decisão
     * 2026-11-03).
     */
    private fun supportedBackPreviewSize(context: Context): Size {
        val modes = CapabilitiesProbe.sessionModes(context, Facing.BACK)
        val mode = SessionModePicker.pick(modes)
            ?: throw AssertionError("câmera back não anuncia modo 16:9 até 1920x1080: $modes")
        Log.i(TAG, "modo do smoke: ${mode.width}x${mode.height}@${mode.fps}")
        return Size(mode.width, mode.height)
    }

    /** [LifecycleOwner] sem Activity: o teste roda sem activity visível. */
    private class TestLifecycleOwner : LifecycleOwner {
        val registry = LifecycleRegistry(this)

        // lifecycle-common 2.6.2 é interface Kotlin: a propriedade é `lifecycle`.
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
        private const val TAG = "CameraBackendSmokeTest"

        /** Duração do stream no smoke (cartão: 10 s). */
        private const val RUN_MS = 10_000L

        /** Margem conservadora: ~meio segundo de quadros a 30 fps. */
        private const val MIN_FRAMES = 15
    }
}
