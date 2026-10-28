package com.localcamera.spike.s3

import android.content.Context
import android.hardware.camera2.CaptureRequest
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

/** T1: CameraX Preview + SurfaceProvider alimentando a Surface do encoder (1080p30). */
class T1CameraXTest(ctx: Context, seconds: Long) : SpikeTest(ctx, seconds) {

    override val id = "T1"

    private var encoder: H264Encoder? = null
    private var provider: ProcessCameraProvider? = null
    private var preview: Preview? = null
    private var camera: Camera? = null
    private var surface: Surface? = null
    private var providerFuture: ListenableFuture<ProcessCameraProvider>? = null

    override fun doStart() {
        val enc = H264Encoder(id, 1920, 1080, 30)
        enc.start()
        encoder = enc
        val s = enc.inputSurface
        if (s == null) {
            S3Log.e("$id error=no_input_surface")
            stop()
            return
        }
        surface = s

        val fut = ProcessCameraProvider.getInstance(ctx)
        providerFuture = fut
        fut.addListener({
            try {
                val p = fut.get()
                provider = p
                val builder = Preview.Builder().setTargetResolution(Size(1920, 1080))
                Camera2Interop.Extender(builder)
                    .setCaptureRequestOption(
                        CaptureRequest.CONTROL_AF_MODE,
                        CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_VIDEO
                    )
                val pv = builder.build()
                preview = pv
                pv.setSurfaceProvider(object : Preview.SurfaceProvider {
                    override fun onSurfaceRequested(request: SurfaceRequest) {
                        val res = request.resolution
                        S3Log.i(
                            "$id event=surface_requested res=${res.width}x${res.height} " +
                                "fps=${request.expectedFrameRate?.lower}-${request.expectedFrameRate?.upper} " +
                                "primary=${request.isPrimary}"
                        )
                        request.provideSurface(s, MAIN_EXECUTOR) { result ->
                            S3Log.i("$id event=surface_provided code=${result.resultCode}")
                        }
                        request.addRequestCancellationListener(MAIN_EXECUTOR) {
                            S3Log.i("$id event=surface_canceled")
                        }
                    }
                })
                val cam = p.bindToLifecycle(
                    ctx as LifecycleOwner,
                    CameraSelector.DEFAULT_BACK_CAMERA,
                    pv
                )
                camera = cam
                val camIds = cam.cameraInfo.cameraIdentifier?.cameraIds
                camIds?.firstOrNull()?.let { logRotationHint(it) }
                S3Log.i("$id event=bound cam=$camIds")
            } catch (t: Throwable) {
                S3Log.e("$id error=start ${t.javaClass.simpleName}: ${t.message}")
                mainPost { stop() }
            }
        }, MAIN_EXECUTOR)
    }

    override fun doStop() {
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
        surface = null
    }
}
