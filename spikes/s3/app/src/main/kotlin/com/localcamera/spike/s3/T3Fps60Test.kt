package com.localcamera.spike.s3

import android.content.Context
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CaptureRequest
import android.os.Handler
import android.os.HandlerThread
import android.util.Range
import android.util.Size
import android.view.Surface
import androidx.camera.camera2.interop.Camera2Interop
import androidx.camera.core.Camera
import androidx.camera.core.CameraSelector
import androidx.camera.core.Preview
import androidx.camera.core.SurfaceRequest
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.lifecycle.LifecycleOwner
import com.google.common.util.concurrent.ListenableFuture

/**
 * T3 1080p60: metade do tempo via CameraX (T3a, setTargetFrameRate + CONTROL_AE_TARGET_FPS_RANGE)
 * e metade via Camera2 (T3b, CONTROL_AE_TARGET_FPS_RANGE direto no request).
 */
class T3Fps60Test(ctx: Context, seconds: Long) : SpikeTest(ctx, seconds) {

    override val id = "T3"

    private var phase = 0
    private var encoder: H264Encoder? = null

    // fase 1: CameraX
    private var provider: ProcessCameraProvider? = null
    private var preview: Preview? = null
    private var camera: Camera? = null
    private var camXFuture: ListenableFuture<ProcessCameraProvider>? = null

    // fase 2: Camera2
    private var device: CameraDevice? = null
    private var session: CameraCaptureSession? = null
    private var thread: HandlerThread? = null
    private var handler: Handler? = null

    private val fpsRange60 = Range(30, 60)

    override fun doStart() {
        val back = cameraId(CameraCharacteristics.LENS_FACING_BACK)
        logRotationHint(back)
        logFpsRanges(back)
        startPhase1()
        mainPostDelayed({ switchToPhase2() }, seconds * 500)
    }

    private fun logFpsRanges(camId: String) {
        val ranges = cameraManager()
            .getCameraCharacteristics(camId)
            ?.get(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES)
        val offers60 = ranges?.any { it.upper >= 60 } ?: false
        S3Log.i(
            "$id cam=$camId fpsRanges=${
                ranges?.joinToString(";") { "${it.lower}-${it.upper}" } ?: "n/a"
            } offers60=$offers60"
        )
    }

    // ---------------- fase 1: CameraX (T3a) ----------------

    private fun startPhase1() {
        phase = 1
        val enc = H264Encoder("T3a", 1920, 1080, 60)
        enc.start()
        encoder = enc
        val s = enc.inputSurface ?: run {
            S3Log.e("$id error=no_input_surface"); stop(); return
        }
        val fut = ProcessCameraProvider.getInstance(ctx)
        camXFuture = fut
        fut.addListener({
            if (phase != 1) return@addListener
            try {
                val p = fut.get()
                provider = p
                val builder = Preview.Builder()
                    .setTargetResolution(Size(1920, 1080))
                    .setTargetFrameRate(fpsRange60)
                Camera2Interop.Extender(builder)
                    .setCaptureRequestOption(
                        CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, fpsRange60
                    )
                val pv = builder.build()
                preview = pv
                pv.setSurfaceProvider(object : Preview.SurfaceProvider {
                    override fun onSurfaceRequested(request: SurfaceRequest) {
                        val res = request.resolution
                        S3Log.i(
                            "T3a event=surface_requested res=${res.width}x${res.height} " +
                                "expectedFps=${request.expectedFrameRate}"
                        )
                        request.provideSurface(s, MAIN_EXECUTOR) { r ->
                            S3Log.i("T3a event=surface_provided code=${r.resultCode}")
                        }
                    }
                })
                val cam = p.bindToLifecycle(
                    ctx as LifecycleOwner,
                    CameraSelector.DEFAULT_BACK_CAMERA,
                    pv
                )
                camera = cam
                S3Log.i("T3a event=bound cam=${cam.cameraInfo.cameraIdentifier?.cameraIds}")
            } catch (t: Throwable) {
                S3Log.e("T3a error=start ${t.javaClass.simpleName}: ${t.message}")
                mainPost { if (phase == 1) switchToPhase2() }
            }
        }, MAIN_EXECUTOR)
    }

    private fun stopPhase1() {
        val pv = preview
        preview = null
        camera = null
        try {
            provider?.unbind(pv)
        } catch (t: Throwable) {
            S3Log.w("$id stop-error=unbind ${t.message}")
        }
        encoder?.stop()
        encoder = null
    }

    private fun switchToPhase2() {
        if (phase != 1) return
        S3Log.i("$id event=phase_switch T3a->T3b")
        stopPhase1()
        startPhase2()
    }

    // ---------------- fase 2: Camera2 (T3b) ----------------

    private fun startPhase2() {
        phase = 2
        val enc = H264Encoder("T3b", 1920, 1080, 60)
        enc.start()
        encoder = enc
        val s = enc.inputSurface ?: run {
            S3Log.e("$id error=no_input_surface"); stop(); return
        }
        val ht = HandlerThread("s3-t3b-cam").also { it.start() }
        thread = ht
        handler = Handler(ht.looper)
        val back = cameraId(CameraCharacteristics.LENS_FACING_BACK)
        try {
            cameraManager().openCamera(back, object : CameraDevice.StateCallback() {
                override fun onOpened(d: CameraDevice) {
                    device = d
                    openSession(d, s)
                }

                override fun onDisconnected(d: CameraDevice) {
                    S3Log.e("T3b event=onDisconnected")
                    mainPost { stop() }
                }

                override fun onError(d: CameraDevice, error: Int) {
                    S3Log.e("T3b event=onError code=$error")
                    mainPost { stop() }
                }
            }, handler)
        } catch (t: Throwable) {
            S3Log.e("T3b error=open ${t.javaClass.simpleName}: ${t.message}")
            stop()
        }
    }

    private fun openSession(d: CameraDevice, s: Surface) {
        val h = handler ?: return
        try {
            d.createCaptureSession(
                listOf(s),
                object : CameraCaptureSession.StateCallback() {
                    override fun onConfigured(cs: CameraCaptureSession) {
                        session = cs
                        val req = d.createCaptureRequest(CameraDevice.TEMPLATE_RECORD)
                        req.set(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, fpsRange60)
                        req.set(
                            CaptureRequest.CONTROL_AF_MODE,
                            CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_VIDEO
                        )
                        cs.setRepeatingRequest(req.build(), null, h)
                        S3Log.i("T3b event=session_ready res=1920x1080 fpsRange=30-60")
                    }

                    override fun onConfigureFailed(cs: CameraCaptureSession) {
                        S3Log.e("T3b error=session_configure_failed")
                        mainPost { stop() }
                    }
                },
                h
            )
        } catch (t: Throwable) {
            S3Log.e("T3b error=create_session ${t.javaClass.simpleName}: ${t.message}")
            mainPost { stop() }
        }
    }

    private fun stopPhase2() {
        val ss = session
        session = null
        try {
            ss?.stopRepeating()
        } catch (t: Throwable) {
            S3Log.w("$id stop-error=session ${t.message}")
        }
        try {
            ss?.close()
        } catch (t: Throwable) {
            S3Log.w("$id stop-error=session ${t.message}")
        }
        val d = device
        device = null
        try {
            d?.close()
        } catch (t: Throwable) {
            S3Log.w("$id stop-error=device ${t.message}")
        }
        encoder?.stop()
        encoder = null
        thread?.quitSafely()
        thread = null
    }

    override fun doStop() {
        if (phase == 1) stopPhase1() else if (phase == 2) stopPhase2()
    }
}
