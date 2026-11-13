package com.localcamera.app.encoder

import android.media.MediaCodecInfo
import android.media.MediaCodecList
import android.os.Build
import android.util.Log
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class EncoderCapabilitiesDiagnosticTest {
    @Test
    fun enumerateAvcEncoderCapabilities() {
        enumerateAvcEncoders(MediaCodecList.REGULAR_CODECS)
    }

    @Test
    fun enumerateAllAvcEncoderCapabilities() {
        enumerateAvcEncoders(MediaCodecList.ALL_CODECS)
    }

    private fun enumerateAvcEncoders(kind: Int) {
        val codecs = MediaCodecList(kind).codecInfos
        var avcEncoders = 0

        for (codec in codecs) {
            if (!codec.isEncoder || codec.supportedTypes.none { it.equals(MIME_AVC, ignoreCase = true) }) {
                continue
            }

            avcEncoders++
            try {
                val capabilities = codec.getCapabilitiesForType(MIME_AVC)
                val colorFormats = capabilities.colorFormats?.toList().orEmpty()
                val supportsCbr = capabilities.encoderCapabilities?.isBitrateModeSupported(
                    MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR,
                )
                val supportsSurface = colorFormats.contains(MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
                val supportsDefaultFormat = capabilities.isFormatSupported(probeFormat(EncoderConfig()))
                val acceleration = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    "hardware=${codec.isHardwareAccelerated} softwareOnly=${codec.isSoftwareOnly}"
                } else {
                    "hardware=unknown softwareOnly=unknown"
                }
                val alias = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    "isAlias=${codec.isAlias}"
                } else {
                    "isAlias=unknown"
                }
                Log.i(
                    TAG,
                    "codec=${codec.name} cbr=$supportsCbr surface=$supportsSurface colorFormats=$colorFormats " +
                        "defaultFormatSupported=$supportsDefaultFormat $acceleration $alias",
                )
            } catch (error: Exception) {
                Log.e(TAG, "codec=${codec.name} capability inspection failed", error)
            }
        }

        assertTrue("Expected at least one video/avc encoder to be enumerated", avcEncoders > 0)
    }

    private companion object {
        const val TAG = "EncoderCapabilitiesDiagnosticTest"
        const val MIME_AVC = "video/avc"
    }
}
