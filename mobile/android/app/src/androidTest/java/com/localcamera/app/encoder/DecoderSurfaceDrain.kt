package com.localcamera.app.encoder

import android.media.MediaCodec

/** Recycle decoder outputs and render them into the supplied encoder surface. */
internal object DecoderSurfaceDrain {

    fun drain(codec: MediaCodec, info: MediaCodec.BufferInfo): Int {
        var rendered = 0
        while (true) {
            val index = codec.dequeueOutputBuffer(info, 0)
            when {
                index >= 0 -> {
                    codec.releaseOutputBuffer(index, true)
                    rendered++
                }
                index == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED -> Unit
                else -> return rendered
            }
        }
    }
}
