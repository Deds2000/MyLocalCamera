package com.localcamera.app.protocol

import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject
import org.json.JSONTokener

// ---------------------------------------------------------------------------
// Modelo (mesmos campos dos vetores, camelCase). Campos desconhecidos são
// ignorados na decodificação. Codificadores em MessagesEncode.kt.
// ---------------------------------------------------------------------------

data class ProtocolVersion(val major: Int, val minor: Int)
data class AppInfo(val version: String, val platform: String)
data class DeviceInfo(val id: String, val name: String, val model: String, val androidApi: Int)

/** Autenticação no HELLO: `{kind:"pair", token}` ou `{kind:"device", deviceId, key}`. */
sealed class Auth {
    data class Pair(val token: String) : Auth()
    data class Device(val deviceId: String, val key: String) : Auth()
}

data class CameraMode(val w: Int, val h: Int, val fps: List<Int>)
data class CameraCaps(val id: String, val facing: String, val modes: List<CameraMode>)
data class Caps(
    val cameras: List<CameraCaps> = emptyList(),
    val concurrent: Boolean = false,
    val audio: Boolean = false,
    val codecs: List<String> = emptyList(),
)
data class Hello(val protocol: ProtocolVersion, val app: AppInfo, val device: DeviceInfo, val auth: Auth, val caps: Caps = Caps())
data class PcInfo(val id: String, val name: String, val appVersion: String)
data class Welcome(
    val accepted: Boolean,
    val protocol: ProtocolVersion? = null,
    val sessionId: String? = null,
    val audioTicket: String? = null,
    val pc: PcInfo? = null,
    /** Motivo de recusa: o protocolo não restringe o valor (desconhecido → mantém). */
    val reason: String? = null,
)
data class AudioHello(val sessionId: String, val ticket: String)
data class Pending(val timeoutS: Int)
data class StreamConfig(val stream: Int, val width: Int, val height: Int, val fps: Int, val rotationHint: Int, val facing: String)

/** Valor de SET: string, inteiro, float ou booleano. */
sealed class SetValue {
    data class Str(val value: String) : SetValue()
    data class IntNum(val value: Long) : SetValue()
    data class FloatNum(val value: Double) : SetValue()
    data class Bool(val value: Boolean) : SetValue()
}

data class SetItem(val key: String, val value: SetValue)

/** SET aceita um item `{key,value}` ou uma lista deles. */
sealed class SetPayload {
    data class One(val item: SetItem) : SetPayload()
    data class Many(val items: List<SetItem>) : SetPayload()
}

data class Sides<T>(val back: T, val front: T)
data class CameraState(val mode: String, val active: String)
data class VideoState(val width: Int, val height: Int, val fps: Int, val bitrateKbps: Int)
data class Presentation(val mirror: Sides<Boolean>, val rotation: Sides<Int>)
data class AudioState(val enabled: Boolean, val gain: Double)
data class State(
    val rev: Long,
    val camera: CameraState? = null,
    val video: VideoState? = null,
    val presentation: Presentation? = null,
    val audio: AudioState? = null,
)
data class Stats(val recvFps: Double, val dropped: Long, val queueMs: Long)
data class Bye(val reason: String)

// ---------------------------------------------------------------------------
// Helpers de JSON (org.json). Campo obrigatório ausente ou tipo errado → BadJson.
// ---------------------------------------------------------------------------

private inline fun <reified T : Any> JSONObject.req(key: String): T =
    (opt(key) as? T) ?: throw CodecError.BadJson("campo '$key' ausente ou com tipo errado")

private inline fun <reified T : Any> JSONObject.optT(key: String): T? =
    when (val v = opt(key)) {
        null, JSONObject.NULL -> null
        is T -> v
        else -> throw CodecError.BadJson("campo '$key' com tipo errado")
    }

private fun JSONObject.reqNum(key: String): Number =
    (opt(key) as? Number) ?: throw CodecError.BadJson("campo '$key' ausente ou não-número")

private fun JSONObject.reqInt(key: String): Int = reqNum(key).asInt(key)
private fun JSONObject.reqLong(key: String): Long = reqNum(key).toLong()
private fun JSONObject.reqDouble(key: String): Double = reqNum(key).toDouble()
private fun JSONObject.optInt(key: String): Int? = optT<Number>(key)?.asInt(key)

private fun Number.asInt(key: String): Int =
    if (this is Double && this != this.toLong().toDouble()) {
        throw CodecError.BadJson("campo '$key' não é inteiro")
    } else {
        toLong().toInt()
    }

/** Converte [JSONArray] em lista (org.json não expõe iteradores Kotlin). */
private fun JSONArray.list(): List<Any?> = (0 until length()).map { get(it) }

private fun JSONArray.ints(key: String): List<Int> =
    list().map { el ->
        (el as? Number)?.asInt(key) ?: throw CodecError.BadJson("campo '$key' com tipo errado")
    }

private fun parseRoot(payload: ByteArray): Any {
    val text = payload.toString(Charsets.UTF_8)
    return try {
        JSONTokener(text).nextValue() ?: throw CodecError.BadJson("JSON vazio")
    } catch (e: JSONException) {
        throw CodecError.BadJson("JSON malformado: ${e.message}")
    }
}

private fun parseObject(payload: ByteArray): JSONObject =
    parseRoot(payload) as? JSONObject ?: throw CodecError.BadJson("raiz deve ser objeto")

private fun JSONObject.decodeVersion(): ProtocolVersion =
    ProtocolVersion(reqInt("major"), reqInt("minor"))

// ---------------------------------------------------------------------------
// Decodificadores (payload JSON → modelo).
// ---------------------------------------------------------------------------

fun decodeHello(payload: ByteArray): Hello {
    val o = parseObject(payload)
    val app = o.req<JSONObject>("app")
    val device = o.req<JSONObject>("device")
    val authObj = o.req<JSONObject>("auth")
    val auth = when (val kind = authObj.req<String>("kind")) {
        "pair" -> Auth.Pair(authObj.req<String>("token"))
        "device" -> Auth.Device(authObj.req<String>("deviceId"), authObj.req<String>("key"))
        else -> throw CodecError.BadJson("auth.kind desconhecido: '$kind'")
    }
    val capsObj = o.optT<JSONObject>("caps")
    val cameras = capsObj?.optT<JSONArray>("cameras")?.list()?.map { el ->
        val c = el as? JSONObject ?: throw CodecError.BadJson("câmera não é objeto")
        val modes: Iterable<Any?> = c.optT<JSONArray>("modes")?.list() ?: emptyList()
        CameraCaps(
            c.req<String>("id"),
            c.req<String>("facing"),
            modes.map { m ->
                val j = m as? JSONObject ?: throw CodecError.BadJson("modo não é objeto")
                val fps = j.req<JSONArray>("fps")
                CameraMode(j.reqInt("w"), j.reqInt("h"), fps.ints("fps"))
            },
        )
    } ?: emptyList()
    return Hello(
        protocol = o.req<JSONObject>("protocol").decodeVersion(),
        app = AppInfo(app.req<String>("version"), app.req<String>("platform")),
        device = DeviceInfo(
            id = device.req<String>("id"),
            name = device.req<String>("name"),
            model = device.optT<String>("model") ?: "",
            androidApi = device.optInt("androidApi") ?: 0,
        ),
        auth = auth,
        caps = Caps(
            cameras = cameras,
            concurrent = capsObj?.optT<Boolean>("concurrent") ?: false,
            audio = capsObj?.optT<Boolean>("audio") ?: false,
            codecs = capsObj?.optT<JSONArray>("codecs")?.list()?.map { el ->
                el as? String ?: throw CodecError.BadJson("campo 'codecs' com tipo errado")
            } ?: emptyList(),
        ),
    )
}

fun decodeWelcome(payload: ByteArray): Welcome {
    val o = parseObject(payload)
    return Welcome(
        accepted = o.req<Boolean>("accepted"),
        protocol = o.optT<JSONObject>("protocol")?.decodeVersion(),
        sessionId = o.optT<String>("sessionId"),
        audioTicket = o.optT<String>("audioTicket"),
        pc = o.optT<JSONObject>("pc")?.let {
            PcInfo(it.req<String>("id"), it.req<String>("name"), it.req<String>("appVersion"))
        },
        reason = o.optT<String>("reason"),
    )
}

fun decodeAudioHello(payload: ByteArray): AudioHello {
    val o = parseObject(payload)
    return AudioHello(o.req<String>("sessionId"), o.req<String>("ticket"))
}

fun decodePending(payload: ByteArray): Pending =
    Pending(parseObject(payload).reqInt("timeoutS"))

fun decodeStreamConfig(payload: ByteArray): StreamConfig {
    val o = parseObject(payload)
    return StreamConfig(
        o.reqInt("stream"),
        o.reqInt("width"),
        o.reqInt("height"),
        o.reqInt("fps"),
        o.reqInt("rotationHint"),
        o.req<String>("facing"),
    )
}

fun decodeSet(payload: ByteArray): SetPayload =
    when (val root = parseRoot(payload)) {
        is JSONObject -> SetPayload.One(decodeSetItem(root))
        is JSONArray -> SetPayload.Many(
            root.list().map {
                it as? JSONObject ?: throw CodecError.BadJson("item de SET não é objeto")
            }.map(::decodeSetItem)
        )
        else -> throw CodecError.BadJson("SET deve ser objeto ou lista")
    }

private fun decodeSetItem(o: JSONObject): SetItem =
    SetItem(o.req<String>("key"), decodeSetValue(o))

private fun decodeSetValue(o: JSONObject): SetValue =
    when (val v = o.opt("value")) {
        null, JSONObject.NULL -> throw CodecError.BadJson("campo 'value' ausente")
        is String -> SetValue.Str(v)
        is Boolean -> SetValue.Bool(v)
        is Number ->
            if (v is Double && v != v.toLong().toDouble()) SetValue.FloatNum(v)
            else SetValue.IntNum(v.toLong())
        else -> throw CodecError.BadJson("campo 'value' com tipo não suportado")
    }

fun decodeState(payload: ByteArray): State {
    val o = parseObject(payload)
    return State(
        rev = o.reqLong("rev"),
        camera = o.optT<JSONObject>("camera")?.let {
            CameraState(it.req<String>("mode"), it.req<String>("active"))
        },
        video = o.optT<JSONObject>("video")?.let {
            VideoState(
                it.reqInt("width"),
                it.reqInt("height"),
                it.reqInt("fps"),
                it.reqInt("bitrateKbps"),
            )
        },
        presentation = o.optT<JSONObject>("presentation")?.let { p ->
            Presentation(
                mirror = p.req<JSONObject>("mirror").let {
                    Sides(it.req<Boolean>("back"), it.req<Boolean>("front"))
                },
                rotation = p.req<JSONObject>("rotation").let {
                    Sides(it.reqInt("back"), it.reqInt("front"))
                },
            )
        },
        audio = o.optT<JSONObject>("audio")?.let {
            AudioState(it.req<Boolean>("enabled"), it.reqDouble("gain"))
        },
    )
}

fun decodeStats(payload: ByteArray): Stats {
    val o = parseObject(payload)
    return Stats(o.reqDouble("recvFps"), o.reqLong("dropped"), o.reqLong("queueMs"))
}

fun decodeBye(payload: ByteArray): Bye =
    Bye(parseObject(payload).req<String>("reason"))
