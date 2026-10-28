package com.localcamera.spike.s3

import android.content.Context
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CaptureRequest
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.view.Surface
import androidx.camera.core.CameraInfo
import androidx.camera.core.CameraProvider
import androidx.camera.lifecycle.ProcessCameraProvider
import com.google.common.util.concurrent.ListenableFuture
import java.util.concurrent.Executors

/**
 * T4 câmeras simultâneas: consulta CameraX (getAvailableConcurrentCameraInfos) e
 * CameraManager.getConcurrentCameraIds(); se suportado, abre traseira+frontal com
 * dois MediaCodecs (dois streams) e registra a resolução máxima aceita por stream.
 */
class T4DualTest(ctx: Context, seconds: Long) : SpikeTest(ctx, seconds) {

    override val id = "T4"

    private var thread: HandlerThread? = null
    private var handler: Handler? = null
    private var cxFuture: ListenableFuture<ProcessCameraProvider>? = null

    private class Stream(val label: String) {
        var camId: String = ""
        var device: CameraDevice? = null
        var session: CameraCaptureSession? = null
        var encoder: H264Encoder? = null
        var sizeIdx = 0
    }

    private val back = Stream("T4b")
    private val front = Stream("T4f")
    private var opened = 0
    private val sizes = listOf(1920 to 1080, 1280 to 720, 640 to 480)

    override fun doStart() {
        val cm = cameraManager()
        val groups: Set<Set<String>> =
            if (Build.VERSION.SDK_INT >= 31) cm.concurrentCameraIds else emptySet()
        S3Log.i("$id concurrentCameraIds=$groups")
        val backId = cameraId(CameraCharacteristics.LENS_FACING_BACK)
        val frontId = cameraId(CameraCharacteristics.LENS_FACING_FRONT)
        back.camId = backId
        front.camId = frontId
        logRotationHint(backId)
        logRotationHint(frontId)

        val fut = ProcessCameraProvider.getInstance(ctx)
        cxFuture = fut
        fut.addListener({
            try {
                val p: ProcessCameraProvider = fut.get()
                val cxIds: List<List<String>> = p.availableConcurrentCameraInfos
                    .map { g: List<CameraInfo> ->
                        g.map { c: CameraInfo ->
                            c.cameraIdentifier?.cameraIds?.firstOrNull()
                                ?: c.cameraIdentifier?.internalId ?: "?"
                        }
                    }
                S3Log.i(
                    "$id cameraxGroups=$cxIds " +
                        "concurrentModeOn=${p.isConcurrentCameraModeOn}"
                )
                val cam2Pair = groups.any { g: Set<String> -> backId in g && frontId in g }
                val cxPair = cxIds.any { g: List<String> -> backId in g && frontId in g }
                if (!cam2Pair && !cxPair) {
                    S3Log.i(
                        "$id dual=sem suporte back=$backId front=$frontId " +
                            "camera2Vazio=${groups.isEmpty()} cameraxVazio=${cxIds.isEmpty()}"
                    )
                    mainPost { stop() }
                    return@addListener
                }
                S3Log.i("$id dual=suportado cam2Pair=$cam2Pair cxPair=$cxPair iniciando")
                startDual()
            } catch (t: Throwable) {
                S3Log.e("$id error=cx_query ${t.javaClass.simpleName}: ${t.message}")
                mainPost { stop() }
            }
        }, MAIN_EXECUTOR)
    }

    private fun startDual() {
        val ht = HandlerThread("s3-t4-cam").also { it.start() }
        thread = ht
        handler = Handler(ht.looper)
        opened = 0
        openOne(back)
        openOne(front)
    }

    private fun openOne(s: Stream) {
        val h = handler ?: return
        try {
            cameraManager().openCamera(
                s.camId,
                object : CameraDevice.StateCallback() {
                    override fun onOpened(d: CameraDevice) {
                        s.device = d
                        opened++
                        S3Log.i("$id event=opened cam=${s.camId} (${s.label})")
                        if (opened == 2) {
                            startEncoders()
                            probeConcurrentConfig()
                            startSessionFor(back)
                            startSessionFor(front)
                        }
                    }

                    override fun onDisconnected(d: CameraDevice) {
                        S3Log.e("$id event=onDisconnected cam=${s.camId}")
                        mainPost { stop() }
                    }

                    override fun onError(d: CameraDevice, error: Int) {
                        S3Log.e("$id event=open_error cam=${s.camId} code=$error")
                        mainPost { stop() }
                    }
                },
                h
            )
        } catch (t: Throwable) {
            S3Log.e("$id error=open ${t.javaClass.simpleName}: ${t.message}")
            mainPost { stop() }
        }
    }

    /** API 31+: valida a configuração de sessão concorrente antes de criá-las. */
    private fun probeConcurrentConfig() {
        if (Build.VERSION.SDK_INT < 31) return
        try {
            val b = back.device ?: return
            val f = front.device ?: return
            val cfgB = sessionConfigFor(b) ?: return
            val cfgF = sessionConfigFor(f) ?: return
            val supported = cameraManager().isConcurrentSessionConfigurationSupported(
                mapOf(back.camId to cfgB, front.camId to cfgF)
            )
            S3Log.i("$id concurrentConfigSupported=$supported res=1920x1080/1920x1080")
        } catch (t: Throwable) {
            S3Log.w("$id concurrentConfig-probe-error=${t.message}")
        }
    }

    private fun sessionConfigFor(d: CameraDevice):
        android.hardware.camera2.params.SessionConfiguration? {
        val enc = if (d.id == back.camId) back.encoder else front.encoder
        val surface = enc?.inputSurface ?: return null
        val noop = object : CameraCaptureSession.StateCallback() {
            override fun onConfigured(session: CameraCaptureSession) = Unit
            override fun onConfigureFailed(session: CameraCaptureSession) = Unit
        }
        return android.hardware.camera2.params.SessionConfiguration(
            android.hardware.camera2.params.SessionConfiguration.SESSION_REGULAR,
            listOf(android.hardware.camera2.params.OutputConfiguration(surface)),
            Executors.newSingleThreadExecutor(),
            noop
        )
    }

    private fun startEncoders() {
        startEncoderFor(back)
        startEncoderFor(front)
    }

    private fun startEncoderFor(s: Stream) {
        val size = sizes[s.sizeIdx]
        val enc = H264Encoder(s.label, size.first, size.second, 30)
        try {
            enc.start()
        } catch (t: Throwable) {
            S3Log.e("$id stream=${s.label} error=encoder ${t.message}")
            mainPost { stop() }
            return
        }
        s.encoder = enc
        if (enc.inputSurface == null) {
            S3Log.e("$id stream=${s.label} error=no_input_surface")
            mainPost { stop() }
            return
        }
    }

    private fun startSessionFor(s: Stream) {
        val dev = s.device ?: run {
            S3Log.e("$id stream=${s.label} error=no_device"); mainPost { stop() }; return
        }
        val surface = s.encoder?.inputSurface ?: run {
            S3Log.e("$id stream=${s.label} error=no_input_surface"); mainPost { stop() }; return
        }
        val hdl = handler ?: return
        try {
            dev.createCaptureSession(listOf(surface), sessionCallback(dev, s), hdl)
        } catch (t: Throwable) {
            S3Log.e("$id stream=${s.label} error=create_session ${t.message}")
            mainPost { stop() }
        }
    }

    private fun sessionCallback(dev: CameraDevice, s: Stream) =
        object : CameraCaptureSession.StateCallback() {
            override fun onConfigured(cs: CameraCaptureSession) {
                s.session = cs
                val req = dev.createCaptureRequest(CameraDevice.TEMPLATE_RECORD)
                req.set(
                    CaptureRequest.CONTROL_AF_MODE,
                    CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_VIDEO
                )
                try {
                    cs.setRepeatingRequest(req.build(), null, handler)
                    val size = sizes[s.sizeIdx]
                    val w = size.first
                    val h = size.second
                    S3Log.i(
                        "$id stream=${s.label} ready res=${w}x${h} " +
                            "maxAccepted=${if (s.sizeIdx == 0) "${w}x${h}" else "below ${w}x${h}"}"
                    )
                } catch (t: Throwable) {
                    S3Log.e("$id stream=${s.label} error=repeating ${t.message}")
                    mainPost { stop() }
                }
            }

            override fun onConfigureFailed(cs: CameraCaptureSession) {
                if (s.sizeIdx < sizes.lastIndex) {
                    S3Log.w("$id stream=${s.label} res_fail tentando menor")
                    s.encoder?.stop()
                    s.encoder = null
                    s.sizeIdx++
                    startEncoderFor(s)
                    startSessionFor(s)
                } else {
                    S3Log.e("$id stream=${s.label} error=all_res_fail (<=640x480)")
                    mainPost { stop() }
                }
            }
        }

    override fun doStop() {
        for (s in listOf(back, front)) {
            val ss = s.session
            s.session = null
            try {
                ss?.stopRepeating()
            } catch (t: Throwable) {
                S3Log.w("$id stop-error=${s.label} ${t.message}")
            }
            try {
                ss?.close()
            } catch (t: Throwable) {
                S3Log.w("$id stop-error=${s.label} ${t.message}")
            }
            val d = s.device
            s.device = null
            try {
                d?.close()
            } catch (t: Throwable) {
                S3Log.w("$id stop-error=${s.label} ${t.message}")
            }
            s.encoder?.stop()
            s.encoder = null
        }
        thread?.quitSafely()
        thread = null
    }
}
