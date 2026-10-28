package com.localcamera.spike.s3

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaCodecList
import android.media.MediaFormat
import android.os.SystemClock
import android.view.Surface
import java.lang.reflect.Array
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicLong

/**
 * Encoder H.264 (video/avc) com entrada via Surface do MediaCodec.
 * CBR 8 Mbps, I-frame 1 s, KEY_LOW_LATENCY quando suportado, sem B-frames
 * (caminho de câmera/low-latency do C2; sem API pública de B-frames no MediaCodec).
 * Emite 1 linha CSV por segundo no logcat (tag S3).
 */
class H264Encoder(
    val label: String,
    val width: Int,
    val height: Int,
    val fps: Int,
    private val bitrateBps: Int = 8_000_000,
) {
    private var codec: MediaCodec? = null
    private var outThread: Thread? = null
    private var statsThread: Thread? = null

    @Volatile
    var inputSurface: Surface? = null
        private set

    @Volatile
    var encoderName: String = "n/a"
        private set

    @Volatile
    var fatal: Boolean = false
        private set

    private val t0 = SystemClock.elapsedRealtime()
    private val frameWindow = AtomicInteger(0)
    private val bytesWindow = AtomicLong(0)
    private val kfWindow = AtomicInteger(0)
    private val firstKeyframeLogged = AtomicBoolean(false)
    private val info = MediaCodec.BufferInfo()

    fun start() {
        val c = MediaCodec.createEncoderByType("video/avc")
        codec = c
        encoderName = c.name
        val caps = avcCapabilities(c.name)
        val lowLatency = probeLowLatency(caps)
        logCapabilities(c, caps, lowLatency)

        val fmt = MediaFormat.createVideoFormat("video/avc", width, height)
        fmt.setInteger(MediaFormat.KEY_BIT_RATE, bitrateBps)
        // AOSP: key "bitrate-mode" (C2); CBR = 2 (MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR).
        fmt.setInteger(
            "bitrate-mode",
            MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR
        )
        fmt.setInteger(MediaFormat.KEY_FRAME_RATE, fps)
        fmt.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1)
        fmt.setInteger(
            MediaFormat.KEY_COLOR_FORMAT,
            MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface
        )
        if (lowLatency) fmt.setInteger(MediaFormat.KEY_LOW_LATENCY, 1)

        c.configure(fmt, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
        // Criar antes de start para a Surface já existir quando a câmera anexar.
        val surface = c.createInputSurface()
        c.start()
        inputSurface = surface
        outThread = Thread({ outputLoop(c) }, "s3-enc-$label").apply { start() }
        statsThread = Thread({ statsLoop() }, "s3-stats-$label").apply { start() }
        S3Log.i(
            "$label started res=${width}x${height} fps=$fps bitrate=$bitrateBps " +
                "name=$encoderName lowLatency=$lowLatency"
        )
    }

    /**
     * FEATURE_LOW_LATENCY: int 128 na API 21-35; String "low-latency" na API 36+.
     * Reflection para compilar contra 36 e rodar em qualquer API >= 24.
     */
    private fun avcCapabilities(name: String):
        MediaCodecInfo.CodecCapabilities {
        val infos = MediaCodecList(MediaCodecList.ALL_CODECS).codecInfos
        return infos.firstOrNull { it.name == name }
            ?.getCapabilitiesForType("video/avc")
            ?: infos.firstOrNull { it.name.endsWith(".avc.encoder") }
                ?.getCapabilitiesForType("video/avc")
            ?: throw IllegalStateException("no avc encoder info found")
    }

    private fun probeLowLatency(caps: MediaCodecInfo.CodecCapabilities): Boolean {
        return try {
            val cls = capsClass()
            if (cls.methods.any {
                    it.name == "isFeatureSupported" &&
                        it.parameterTypes.get(0) == Int::class.javaPrimitiveType
                }
            ) {
                cls.getMethod("isFeatureSupported", Int::class.javaPrimitiveType)
                    .invoke(caps, 128) as Boolean
            } else {
                val key = cls.getField("FEATURE_LowLatency").get(null) as String
                cls.getMethod("isFeatureSupported", String::class.java)
                    .invoke(caps, key) as Boolean
            }
        } catch (t: Throwable) {
            S3Log.w("$label caps-error=lowlatency ${t.message}")
            false
        }
    }

    /**
     * profileLevels: ProfileLevel (getters) na API <=35; CodecProfileLevel (fields) na 36+.
     * Reflection + nomes dos consts AVC* (valores inlined, estáveis).
     */
    private fun logCapabilities(
        c: MediaCodec,
        caps: MediaCodecInfo.CodecCapabilities,
        lowLatency: Boolean
    ) {
        try {
            val pls = capsClass().getField("profileLevels").get(caps)
            val profiles = (0 until Array.getLength(pls)).joinToString(";") { i ->
                val pl = Array.get(pls, i)
                val pc = pl.javaClass
                val prof = (if (pc.declaredFields.any { it.name == "profile" })
                    pc.getField("profile").get(pl)
                else pc.getMethod("getProfile").invoke(pl)) as Int
                val lvl = (if (pc.declaredFields.any { it.name == "level" })
                    pc.getField("level").get(pl)
                else pc.getMethod("getLevel").invoke(pl)) as Int
                "${profileName(prof)}($prof)/L$lvl"
            }
            S3Log.i("$label enc_name=${c.name} profiles=$profiles lowLatency=$lowLatency")
        } catch (t: Throwable) {
            S3Log.w("$label caps-error=${t.message}")
        }
    }

    private fun capsClass(): Class<*> =
        Class.forName("android.media.MediaCodecInfo\$CodecCapabilities")

    private fun profileName(p: Int): String = when (p) {
        MediaCodecInfo.CodecProfileLevel.AVCProfileBaseline -> "Baseline"
        MediaCodecInfo.CodecProfileLevel.AVCProfileConstrainedBaseline -> "ConstrBaseline"
        MediaCodecInfo.CodecProfileLevel.AVCProfileExtended -> "Extended"
        MediaCodecInfo.CodecProfileLevel.AVCProfileMain -> "Main"
        MediaCodecInfo.CodecProfileLevel.AVCProfileHigh -> "High"
        MediaCodecInfo.CodecProfileLevel.AVCProfileConstrainedHigh -> "ConstrHigh"
        MediaCodecInfo.CodecProfileLevel.AVCProfileHigh10 -> "High10"
        MediaCodecInfo.CodecProfileLevel.AVCProfileHigh422 -> "High422"
        MediaCodecInfo.CodecProfileLevel.AVCProfileHigh444 -> "High444"
        else -> "p"
    }

    private fun outputLoop(c: MediaCodec) {
        while (!fatal) {
            val idx = try {
                c.dequeueOutputBuffer(info, 10_000)
            } catch (t: IllegalStateException) {
                break
            }
            when {
                idx == MediaCodec.INFO_TRY_AGAIN_LATER -> Unit
                idx == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED ->
                    S3Log.i("$label outputFormatChanged")
                idx >= 0 -> {
                    if (info.flags and KEY_FRAME != 0) {
                        kfWindow.incrementAndGet()
                        if (firstKeyframeLogged.compareAndSet(false, true)) {
                            val ms = SystemClock.elapsedRealtime() - t0
                            S3Log.i("$label timeToFirstKeyframe_ms=$ms")
                        }
                    }
                    frameWindow.incrementAndGet()
                    bytesWindow.addAndGet(info.size.toLong())
                    c.releaseOutputBuffer(idx, false)
                }
                else -> {
                    S3Log.e("$label error=codec_error code=$idx")
                    fatal = true
                }
            }
        }
    }

    private fun statsLoop() {
        while (!fatal) {
            try {
                Thread.sleep(1000)
            } catch (t: InterruptedException) {
                break
            }
            val f = frameWindow.getAndSet(0)
            val b = bytesWindow.getAndSet(0)
            val k = kfWindow.getAndSet(0)
            val t = SystemClock.elapsedRealtime() - t0
            S3Log.csv(label, t, f, b, k, encoderName)
        }
    }

    fun stop() {
        if (codec == null) return
        fatal = true
        try {
            outThread?.join(500)
            statsThread?.join(200)
        } catch (t: InterruptedException) {
            Thread.currentThread().interrupt()
        }
        val c = codec ?: return
        codec = null
        try {
            c.stop()
        } catch (t: Throwable) {
            S3Log.w("$label stop-error=${t.message}")
        }
        try {
            c.release()
        } catch (t: Throwable) {
            S3Log.w("$label release-error=${t.message}")
        }
        val s = inputSurface
        inputSurface = null
        try {
            s?.release()
        } catch (t: Throwable) {
            S3Log.w("$label surface-release-error=${t.message}")
        }
    }

    companion object {
        // = 1 em todas as APIs (antigo MediaCodec.BufferFlag.KEY_FRAME).
        const val KEY_FRAME: Int = MediaCodec.BUFFER_FLAG_KEY_FRAME
    }
}
