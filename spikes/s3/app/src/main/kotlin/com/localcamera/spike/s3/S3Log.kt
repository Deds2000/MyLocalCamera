package com.localcamera.spike.s3

import android.util.Log
import java.util.Locale

/** Log tag S3 no logcat. CSV: teste,t_ms,fps_inst,bytes,keyframe,enc_name */
object S3Log {
    const val TAG = "S3"

    fun i(msg: String) = Log.i(TAG, msg)
    fun w(msg: String) = Log.w(TAG, msg)
    fun e(msg: String) = Log.e(TAG, msg)

    fun csv(test: String, tMs: Long, fpsInst: Int, bytes: Long, keyframe: Int, encName: String) {
        val fps = String.format(Locale.US, "%.1f", fpsInst.toDouble())
        Log.i(TAG, "$test,$tMs,$fps,$bytes,$keyframe,$encName")
    }
}
