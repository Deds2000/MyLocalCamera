package com.localcamera.spike.s3

import android.content.Context
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraManager
import android.os.Handler
import android.os.Looper
import java.util.concurrent.Executor
import java.util.concurrent.Executors

/** Base comum dos 4 testes: duração com parada automática + parada manual. */
abstract class SpikeTest(val ctx: Context, val seconds: Long) {

    protected abstract val id: String

    private val main = Handler(Looper.getMainLooper())
    private val timeoutRunnable = Runnable {
        S3Log.i("$id event=timeout seconds=$seconds")
        stop()
    }

    @Volatile
    private var active = false

    fun start() {
        if (active) return
        active = true
        S3Log.i("$id event=start seconds=$seconds")
        main.postDelayed(timeoutRunnable, seconds * 1000L)
        doStart()
    }

    fun stop() {
        if (!active) return
        active = false
        main.removeCallbacks(timeoutRunnable)
        S3Log.i("$id event=stop")
        doStop()
    }

    protected abstract fun doStart()

    protected abstract fun doStop()

    protected fun mainPost(r: Runnable): Unit {
        main.post(r)
    }

    protected fun mainPostDelayed(r: Runnable, ms: Long): Unit {
        main.postDelayed(r, ms)
    }

    protected fun cameraManager(): CameraManager =
        ctx.getSystemService(Context.CAMERA_SERVICE) as CameraManager

    protected fun cameraId(facing: Int): String {
        val cm = cameraManager()
        return cm.cameraIdList.firstOrNull {
            cm.getCameraCharacteristics(it)?.get(CameraCharacteristics.LENS_FACING) == facing
        } ?: cm.cameraIdList.first()
    }

    /**
     * rotationHint = (sensorOrientation - displayRotation + 360) % 360.
     * Retratro: displayRotation=0; paisagem: displayRotation=90 (convenção de webcam).
     */
    protected fun logRotationHint(camId: String) {
        try {
            val sensor = cameraManager()
                .getCameraCharacteristics(camId)
                ?.get(CameraCharacteristics.SENSOR_ORIENTATION) ?: 0
            val portrait = (sensor + 360) % 360
            val landscape = (sensor - 90 + 360) % 360
            S3Log.i("$id rotationHint cam=$camId sensor=$sensor portrait=$portrait landscape=$landscape")
        } catch (t: Throwable) {
            S3Log.w("$id rotationHint-error=${t.message}")
        }
    }

    companion object {
        val MAIN_EXECUTOR: Executor = object : Executor {
            override fun execute(command: Runnable) {
                Handler(Looper.getMainLooper()).post(command)
            }
        }
    }
}
