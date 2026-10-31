package com.localcamera.app.service

import android.content.Context
import android.os.Build
import com.localcamera.app.camera.CapabilitiesProbe
import com.localcamera.app.protocol.AppInfo
import com.localcamera.app.protocol.Auth
import com.localcamera.app.protocol.CameraCaps
import com.localcamera.app.protocol.CameraMode
import com.localcamera.app.protocol.Caps
import com.localcamera.app.protocol.DeviceInfo
import com.localcamera.app.protocol.Hello
import com.localcamera.app.protocol.ProtocolVersion
import com.localcamera.app.store.PairingStore
import com.localcamera.app.uri.PairingUri
import org.json.JSONObject

/**
 * Montagem do HELLO (F2-T12): protocolo 1.0 + app + device + `auth`
 * (`deviceKey` se já pareado com "Permitir sempre", senão token do QR) +
 * `caps` da sonda (F2-T06). Extraído da `StreamSession` para mantê-la
 * ≤ 300 linhas (regra do cartão).
 *
 * Segredos: `token`/`deviceKey` entram **só** no payload — nunca em logs,
 * eventos ou `toString` (regra de ouro do repo).
 */
internal object HelloBuilder {

    fun build(context: Context, target: PairingUri, store: PairingStore): Hello {
        val pcId = target.pc.orEmpty()
        val auth = store.deviceKey(pcId)?.let { key ->
            Auth.Device(store.deviceId(), key.toHex())
        } ?: Auth.Pair(target.token)
        return Hello(
            protocol = ProtocolVersion(1, 0),
            app = AppInfo(appVersion(context), "android"),
            device = DeviceInfo(store.deviceId(), Build.MODEL, Build.MODEL, Build.VERSION.SDK_INT),
            auth = auth,
            caps = parseCaps(CapabilitiesProbe.probe(context)),
        )
    }

    private fun appVersion(context: Context): String = try {
        context.packageManager.getPackageInfo(context.packageName, 0).versionName ?: "1.0"
    } catch (e: Exception) {
        "1.0"
    }

    /** JSON `caps` da sonda (F2-T06) → modelo do protocolo p/ o `encodeHello`. */
    private fun parseCaps(json: String): Caps = try {
        val o = JSONObject(json)
        val cameras = o.optJSONArray("cameras")?.let { arr ->
            (0 until arr.length()).mapNotNull { i ->
                val c = arr.optJSONObject(i) ?: return@mapNotNull null
                val modes = c.optJSONArray("modes")?.let { m ->
                    (0 until m.length()).mapNotNull { j ->
                        val mm = m.optJSONObject(j) ?: return@mapNotNull null
                        val fps = mm.optJSONArray("fps")?.let { f ->
                            (0 until f.length()).map { k -> f.optInt(k) }
                        } ?: emptyList()
                        CameraMode(mm.optInt("w"), mm.optInt("h"), fps)
                    }
                } ?: emptyList()
                CameraCaps(c.optString("id"), c.optString("facing"), modes)
            }
        } ?: emptyList()
        val codecs = o.optJSONArray("codecs")?.let { a ->
            (0 until a.length()).map { a.optString(it) }
        } ?: emptyList()
        Caps(cameras, o.optBoolean("concurrent"), o.optBoolean("audio"), codecs)
    } catch (e: Exception) {
        Caps()
    }

    private fun ByteArray.toHex(): String = buildString(size * 2) {
        for (b in this@toHex) {
            append(HEX[(b.toInt() shr 4) and 0xF])
            append(HEX[b.toInt() and 0xF])
        }
    }

    private const val HEX = "0123456789abcdef"
}
