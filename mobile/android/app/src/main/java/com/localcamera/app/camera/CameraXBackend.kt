package com.localcamera.app.camera

import android.content.Context
import android.hardware.camera2.CaptureRequest
import android.util.Log
import android.util.Range
import android.util.Size
import android.view.Surface
import androidx.camera.camera2.interop.Camera2Interop
import androidx.camera.core.Camera
import androidx.camera.core.CameraSelector
import androidx.camera.core.CameraState
import androidx.camera.core.Preview
import androidx.camera.core.SurfaceRequest
import androidx.camera.core.resolutionselector.AspectRatioStrategy
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.core.content.ContextCompat
import androidx.lifecycle.LifecycleOwner
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.Executor

/**
 * [CameraBackend] via CameraX 1.6.2 (veredito S3 — modo single):
 * - `Preview` + `SurfaceProvider` alimentam a Surface do encoder (S3 T1:
 *   1080p30 estável, 29,7 fps, jitter <1 por 10 min);
 * - **obrigatório** (armadilha S3): `ResolutionSelector` 16:9 com alvo e
 *   fallback — o `setTargetResolution` depreciado caía em 1088×1088;
 * - fps via `Camera2Interop` (`CONTROL_AE_TARGET_FPS_RANGE` [fps, fps];
 *   S3: 30-30 estável; 60 não existe no aparelho e nunca é oferecido);
 * - `rotationHint` = orientação do sensor (`sensorRotationDegrees`) +
 *   rotação do aparelho (listener `ComponentCallbacks2` + `DisplayManager`);
 * - interrupção (outro app abre a câmera, chamada) → [CameraEvent.Error].
 *
 * Tipos CameraX ficam restritos ao pacote `camera/`: a API pública é
 * [CameraBackend]/[CameraConfig]/[CameraEvent]/[CameraEventCode].
 *
 * Chamadas públicas a partir da main thread (a `StreamingService` do
 * F2-T12); callbacks do CameraX chegam no [executor] (main por padrão).
 */
class CameraXBackend(
    private val context: Context,
    private val lifecycleOwner: LifecycleOwner,
    private val executor: Executor = ContextCompat.getMainExecutor(context),
    private val deviceRotationDeg: () -> Int = { currentDisplayRotation(context) },
) : CameraBackend {

    private val eventListeners = CopyOnWriteArrayList<(CameraEvent) -> Unit>()
    private val rotationListeners = CopyOnWriteArrayList<(Int) -> Unit>()
    private val lock = Any()

    private var provider: ProcessCameraProvider? = null
    private var camera: Camera? = null
    private var preview: Preview? = null
    private var config: CameraConfig? = null
    private var outputSurface: Surface? = null
    private var sensorOrientationDeg: Int? = null
    private var running = false
    private var startedEmitted = false
    private var generation = 0
    private val rotationObserver = DeviceRotationObserver(context) {
        config?.facing?.let { emitHint(it) }
    }

    // ------------------------------ listeners ------------------------------

    override fun onEvent(listener: (CameraEvent) -> Unit) {
        eventListeners.add(listener)
    }

    override fun onRotationHint(listener: (Int) -> Unit) {
        rotationListeners.add(listener)
    }

    // ------------------------------ ciclo de vida ------------------------------

    override fun start(config: CameraConfig, outputSurface: Surface) {
        if (synchronized(lock) { running }) {
            Log.w(TAG, "start() ignorado: já está rodando")
            return
        }
        val gen = synchronized(lock) {
            generation++
            this.config = config
            this.outputSurface = outputSurface
            startedEmitted = false
            running = true
            generation
        }
        val fut = ProcessCameraProvider.getInstance(context)
        fut.addListener({
            if (gen != generation) return@addListener // stop() durante o await
            try {
                val p = fut.get()
                // `this.`: o parâmetro de start() sombreia a propriedade
                // (leitura sem this usaria o valor antigo após stop()).
                val cfg = synchronized(lock) { this.config } ?: return@addListener
                val surf = synchronized(lock) { this.outputSurface } ?: return@addListener
                openCamera(p, cfg, surf, gen)
            } catch (t: Throwable) {
                if (gen != generation) return@addListener
                Log.w(TAG, "start falhou: ${t.javaClass.simpleName}: ${t.message}")
                failStart(CameraEventCode.START_FAILED)
            }
        }, executor)
    }

    override fun stop() {
        val (cfg, pv) = synchronized(lock) {
            val c = config
            val p = preview
            generation++
            running = false
            startedEmitted = false
            config = null
            outputSurface = null
            camera = null
            c to p
        }
        safeUnbind(pv)
        preview = null
        rotationObserver.unregister()
        Log.i(TAG, "parado (facing=${cfg?.facing?.code})")
    }

    override fun switchTo(facing: Facing) {
        val (cfg, pv, p) = synchronized(lock) {
            val c = config
            val v = preview
            Triple(c, v, provider)
        }
        val current = cfg ?: return
        if (current.facing == facing) return
        val surface = synchronized(lock) { outputSurface } ?: return
        if (p == null) {
            Log.w(TAG, "switchTo ignorado: provider indisponível")
            return
        }
        // Fecha a câmera atual (o CLOSED antigo é ignorado via generation);
        // Started volta a emitir quando a nova câmera abrir.
        synchronized(lock) {
            startedEmitted = false
        }
        safeUnbind(pv)
        preview = null
        camera = null
        openCamera(p, current.copy(facing = facing), surface, synchronized(lock) { generation++ })
    }

    /** Libera o singleton [ProcessCameraProvider] (onDestroy do serviço — F2-T12). */
    fun release() {
        stop()
        try {
            ProcessCameraProvider.shutdown()
        } catch (t: Throwable) {
            Log.w(TAG, "release: ${t.message}")
        }
    }

    // ------------------------------ câmera ------------------------------

    private fun openCamera(
        provider: ProcessCameraProvider,
        cfg: CameraConfig,
        surface: Surface,
        gen: Int,
    ) {
        this.provider = provider
        val selector = selectorFor(cfg.facing)
        if (!provider.hasCamera(selector)) {
            failStart(CameraEventCode.CAMERA_UNAVAILABLE)
            return
        }
        val pv = buildPreview(cfg)
        pv.setSurfaceProvider { request -> provideEncoderSurface(request, surface) }
        val cam = try {
            provider.bindToLifecycle(lifecycleOwner, selector, pv)
        } catch (t: SecurityException) {
            failStart(CameraEventCode.PERMISSION_DENIED)
            return
        } catch (t: Throwable) {
            failStart(CameraEventCode.START_FAILED)
            return
        }
        camera = cam
        preview = pv
        sensorOrientationDeg = cam.cameraInfo.sensorRotationDegrees
        cam.cameraInfo.addCameraStateListener(executor) { st -> onCameraState(gen, st) }
        rotationObserver.register()
        emitHint(cfg.facing)
        Log.i(TAG, "aberta facing=${cfg.facing.code} sensor=${sensorOrientationDeg}° gen=$gen")
    }

    /**
     * `Preview` com a resolução correta (armadilha S3): aspecto 16:9 com
     * fallback automático + alvo [CameraConfig.width]×[CameraConfig.height]
     * (regra closest higher-then-lower). Nunca `setTargetResolution`.
     */
    private fun buildPreview(cfg: CameraConfig): Preview {
        val selector = ResolutionSelector.Builder()
            .setAspectRatioStrategy(AspectRatioStrategy.RATIO_16_9_FALLBACK_AUTO_STRATEGY)
            .setResolutionStrategy(
                ResolutionStrategy(
                    Size(cfg.width, cfg.height),
                    ResolutionStrategy.FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER,
                )
            )
            .build()
        val builder = Preview.Builder().setResolutionSelector(selector)
        Camera2Interop.Extender(builder)
            .setCaptureRequestOption(
                CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE,
                Range(cfg.fps, cfg.fps),
            )
        return builder.build()
    }

    private fun provideEncoderSurface(request: SurfaceRequest, surface: Surface) {
        val res = request.resolution
        val fps = request.expectedFrameRate
        Log.i(TAG, "superfície pedida ${res.width}x${res.height} fps=${fps.lower}-${fps.upper}")
        request.provideSurface(surface, executor) { result ->
            Log.i(TAG, "superfície do encoder em uso code=${result.resultCode}")
        }
        request.addRequestCancellationListener(executor) {
            Log.i(TAG, "pedido de superfície cancelado")
        }
    }

    private fun onCameraState(gen: Int, state: CameraState) {
        if (gen != generation) return // estado de câmera anterior (troca/stop)
        val err = state.error
        if (err != null) {
            val code = CameraEventCode.fromCameraState(err.code)
            Log.w(TAG, "erro da câmera code=${err.code} tipo=${err.type}")
            resetInternal()
            emit(CameraEvent.Error(code))
            return
        }
        when (state.type) {
            CameraState.Type.OPEN ->
                if (!startedEmitted) {
                    startedEmitted = true
                    emit(CameraEvent.Started)
                }
            CameraState.Type.CLOSED ->
                if (running) {
                    // Fechada pelo sistema sem nosso stop() (chamada, outro app).
                    Log.w(TAG, "câmera fechada pelo sistema (interrupção)")
                    resetInternal()
                    emit(CameraEvent.Error(CameraEventCode.CAMERA_CLOSED))
                }
            else -> Unit // PENDING_OPEN, OPENING, CLOSING
        }
    }

    private fun failStart(code: String) {
        resetInternal()
        emit(CameraEvent.Error(code))
    }

    /** Limpa o estado e fecha o que estiver aberto (após erro/interrupção). */
    private fun resetInternal() {
        val pv = synchronized(lock) {
            val p = preview
            generation++
            running = false
            startedEmitted = false
            config = null
            outputSurface = null
            camera = null
            p
        }
        safeUnbind(pv)
        preview = null
        rotationObserver.unregister()
    }

    private fun safeUnbind(pv: Preview?) {
        val p = provider ?: return
        if (pv == null) return
        try {
            p.unbind(pv)
        } catch (t: Throwable) {
            Log.w(TAG, "unbind: ${t.message}")
        }
    }

    // ------------------------------ rotação ------------------------------

    private fun emitHint(facing: Facing) {
        val sensor = sensorOrientationDeg ?: return
        val hint = RotationHint.forCamera(facing, sensor, safeDeviceRotation())
        rotationListeners.forEach { it(hint) }
        emit(CameraEvent.Rotated(hint))
    }

    private fun safeDeviceRotation(): Int =
        try {
            deviceRotationDeg()
        } catch (t: Throwable) {
            0
        }

    // ------------------------------ util ------------------------------

    private fun emit(event: CameraEvent) {
        eventListeners.forEach { it(event) }
    }

    private fun selectorFor(facing: Facing): CameraSelector = when (facing) {
        Facing.BACK -> CameraSelector.DEFAULT_BACK_CAMERA
        Facing.FRONT -> CameraSelector.DEFAULT_FRONT_CAMERA
    }

    private companion object {
        private const val TAG = "CameraXBackend"
    }
}
