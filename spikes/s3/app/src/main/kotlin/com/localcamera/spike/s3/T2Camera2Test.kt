package com.localcamera.spike.s3

import android.content.Context
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CaptureRequest
import android.os.Handler
import android.os.HandlerThread
import android.util.Range
import android.view.Surface

/** T2: Camera2 direto (CameraDevice/CaptureSession) na Surface do encoder, 1080p30. */
class T2Camera2Test(ctx: Context, seconds: Long) : SpikeTest(ctx, seconds) {

    override val id = "T2"

    private var encoder: H264Encoder? = null
    private var device: CameraDevice? = null
    private var session: CameraCaptureSession? = null
    private var thread: HandlerThread? = null
    private var handler: Handler? = null

    override fun doStart() {
        val enc = H264Encoder(id, 1920, 1080, 30)
        enc.start()
        encoder = enc
        val surface = enc.inputSurface
        if (surface == null) {
            S3Log.e("$id error=no_input_surface")
            stop()
            return
        }
        val ht = HandlerThread("s3-t2-cam").also { it.start() }
        thread = ht
        handler = Handler(ht.looper)

        val camId = cameraId(CameraCharacteristics.LENS_FACING_BACK)
        logRotationHint(camId)
        try {
            cameraManager().openCamera(camId, object : CameraDevice.StateCallback() {
                override fun onOpened(d: CameraDevice) {
                    device = d
                    openSession(d, surface)
                }

                override fun onDisconnected(d: CameraDevice) {
                    S3Log.e("$id event=onDisconnected")
                    mainPost { stop() }
                }

                override fun onError(d: CameraDevice, error: Int) {
                    S3Log.e("$id event=onError code=$error")
                    mainPost { stop() }
                }
            }, handler)
        } catch (t: Throwable) {
            S3Log.e("$id error=open_camera ${t.javaClass.simpleName}: ${t.message}")
            stop()
        }
    }

    private fun openSession(d: CameraDevice, surface: Surface) {
        val h = handler ?: return
        try {
            d.createCaptureSession(
                listOf(surface),
                object : CameraCaptureSession.StateCallback() {
                    override fun onConfigured(s: CameraCaptureSession) {
                        session = s
                        val req = d.createCaptureRequest(CameraDevice.TEMPLATE_RECORD)
                        req.set(
                            CaptureRequest.CONTROL_AF_MODE,
                            CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE
                        )
                        req.set(CaptureRequest.CONTROL_AE_MODE, CaptureRequest.CONTROL_AE_MODE_ON)
                        req.set(
                            CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE,
                            Range(30, 30)
                        )
                        req.addTarget(surface)
                        s.setRepeatingRequest(req.build(), null, h)
                        S3Log.i("$id event=session_ready res=1920x1080")
                    }

                    override fun onConfigureFailed(s: CameraCaptureSession) {
                        S3Log.e("$id error=session_configure_failed")
                        mainPost { stop() }
                    }
                },
                h
            )
        } catch (t: Throwable) {
            S3Log.e("$id error=create_session ${t.javaClass.simpleName}: ${t.message}")
            mainPost { stop() }
        }
    }

    override fun doStop() {
        val s = session
        session = null
        try {
            s?.stopRepeating()
        } catch (t: Throwable) {
            S3Log.w("$id stop-error=session=${t.message}")
        }
        try {
            s?.close()
        } catch (t: Throwable) {
            S3Log.w("$id stop-error=session=${t.message}")
        }
        val d = device
        device = null
        try {
            d?.close()
        } catch (t: Throwable) {
            S3Log.w("$id stop-error=device=${t.message}")
        }
        encoder?.stop()
        encoder = null
        thread?.quitSafely()
        thread = null
    }
}
