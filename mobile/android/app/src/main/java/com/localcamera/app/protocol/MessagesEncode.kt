package com.localcamera.app.protocol

import org.json.JSONArray
import org.json.JSONObject

// ---------------------------------------------------------------------------
// Codificadores (modelo → payload JSON). Modelo em Messages.kt.
// ---------------------------------------------------------------------------

private fun JSONObject.toUtf8(): ByteArray = toString().toByteArray(Charsets.UTF_8)

private fun JSONArray.toUtf8(): ByteArray = toString().toByteArray(Charsets.UTF_8)

private fun versionJson(v: ProtocolVersion): JSONObject =
    JSONObject().apply { put("major", v.major); put("minor", v.minor) }

fun encodeHello(h: Hello): ByteArray = JSONObject().apply {
    put("protocol", versionJson(h.protocol))
    put("app", JSONObject().apply {
        put("version", h.app.version); put("platform", h.app.platform)
    })
    put("device", JSONObject().apply {
        put("id", h.device.id); put("name", h.device.name)
        put("model", h.device.model); put("androidApi", h.device.androidApi)
    })
    put("auth", JSONObject().apply {
        when (val a = h.auth) {
            is Auth.Pair -> put("kind", "pair").put("token", a.token)
            is Auth.Device -> put("kind", "device").put("deviceId", a.deviceId).put("key", a.key)
        }
    })
    put("caps", JSONObject().apply {
        put("cameras", JSONArray().apply {
            h.caps.cameras.forEach { c ->
                put(JSONObject().apply {
                    put("id", c.id); put("facing", c.facing)
                    put("modes", JSONArray().apply {
                        c.modes.forEach { m ->
                            put(JSONObject().apply {
                                put("w", m.w); put("h", m.h)
                                put("fps", JSONArray().apply { m.fps.forEach { f -> put(f) } })
                            })
                        }
                    })
                })
            }
        })
        put("concurrent", h.caps.concurrent)
        put("audio", h.caps.audio)
        put("codecs", JSONArray().apply { h.caps.codecs.forEach { c -> put(c) } })
    })
}.toUtf8()

fun encodeWelcome(w: Welcome): ByteArray = JSONObject().apply {
    put("accepted", w.accepted)
    w.protocol?.let { put("protocol", versionJson(it)) }
    w.sessionId?.let { put("sessionId", it) }
    w.audioTicket?.let { put("audioTicket", it) }
    w.pc?.let {
        put("pc", JSONObject().apply {
            put("id", it.id); put("name", it.name); put("appVersion", it.appVersion)
        })
    }
    w.reason?.let { put("reason", it) }
}.toUtf8()

fun encodeAudioHello(a: AudioHello): ByteArray = JSONObject().apply {
    put("sessionId", a.sessionId); put("ticket", a.ticket)
}.toUtf8()

fun encodePending(p: Pending): ByteArray =
    JSONObject().apply { put("timeoutS", p.timeoutS) }.toUtf8()

fun encodeStreamConfig(s: StreamConfig): ByteArray = JSONObject().apply {
    put("stream", s.stream); put("width", s.width); put("height", s.height)
    put("fps", s.fps); put("rotationHint", s.rotationHint); put("facing", s.facing)
}.toUtf8()

private fun SetItem.toJson(): JSONObject = JSONObject().apply {
    put("key", key)
    put("value", when (val v = value) {
        is SetValue.Str -> v.value
        is SetValue.IntNum -> v.value
        is SetValue.FloatNum -> v.value
        is SetValue.Bool -> v.value
    })
}

fun encodeSet(s: SetPayload): ByteArray =
    when (s) {
        is SetPayload.One -> s.item.toJson().toUtf8()
        is SetPayload.Many ->
            JSONArray().apply { s.items.forEach { put(it.toJson()) } }.toUtf8()
    }

fun encodeState(st: State): ByteArray = JSONObject().apply {
    put("rev", st.rev)
    st.camera?.let {
        put("camera", JSONObject().apply { put("mode", it.mode); put("active", it.active) })
    }
    st.video?.let {
        put("video", JSONObject().apply {
            put("width", it.width); put("height", it.height)
            put("fps", it.fps); put("bitrateKbps", it.bitrateKbps)
        })
    }
    st.presentation?.let { p ->
        put("presentation", JSONObject().apply {
            put("mirror", JSONObject().apply {
                put("back", p.mirror.back); put("front", p.mirror.front)
            })
            put("rotation", JSONObject().apply {
                put("back", p.rotation.back); put("front", p.rotation.front)
            })
        })
    }
    st.audio?.let {
        put("audio", JSONObject().apply { put("enabled", it.enabled); put("gain", it.gain) })
    }
}.toUtf8()

fun encodeStats(s: Stats): ByteArray = JSONObject().apply {
    put("recvFps", s.recvFps); put("dropped", s.dropped); put("queueMs", s.queueMs)
}.toUtf8()

fun encodeBye(b: Bye): ByteArray =
    JSONObject().apply { put("reason", b.reason) }.toUtf8()
