package com.localcamera.app.protocol

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import java.io.File

/**
 * UT-2.01 / UT-2.02 — codec contra os vetores reais
 * (`docs/protocol-vectors.json`): 8 vetores de frames (leitor) + 24 de JSON
 * (decode + encode de volta) e entrada fragmentada em todos os tamanhos 1..len.
 */
class ProtocolCodecVectorsTest {

    companion object {
        private val vectors: JSONObject = run {
            val f = File("../../../docs/protocol-vectors.json")
            assertTrue("arquivo de vetores não encontrado: rode a partir da raiz do repo", f.exists())
            JSONObject(f.readText(Charsets.UTF_8))
        }
    }

    // UT-2.01 (frames): reader nos 8 vetores + escritor reproduz os bytes.
    @Test
    fun ut201_frameVectors() {
        val arr = vectors.getJSONArray("frames")
        for (i in 0 until arr.length()) {
            val v = arr.getJSONObject(i)
            val name = v.getString("name")
            val ctx = "frame[$name]"
            val bytes = hex(v.getString("hex"))
            val expect = v.getJSONObject("expect")

            val reader = FrameReader()
            val result = reader.feed(bytes)

            if (expect.has("error")) {
                val err = result as? ReaderResult.Error
                    ?: error("$ctx deveria devolver erro, veio $result")
                assertEquals("$ctx erro", expect.getString("error"), err.error.kind)
            } else {
                val ok = result as? ReaderResult.Ok
                    ?: error("$ctx deveria devolver frames, veio $result")
                val expected = expectFrames(expect)
                assertEquals("$ctx quantidade de frames", expected.size, ok.frames.size)
                ok.frames.zip(expected) { f, e -> assertFrame(f, e, ctx) }
                assertEquals("$ctx pendingBytes", expect.getInt("pendingBytes"), reader.pendingBytes)
                // O escritor re-encodifica exatamente os bytes do vetor.
                if (expected.isNotEmpty()) {
                    val reencoded = ok.frames.fold(ByteArray(0)) { acc, f -> acc + FrameWriter.write(f) }
                    assertEquals("$ctx re-encode", toHex(bytes), toHex(reencoded))
                }
            }
        }
    }

    // UT-2.01 (JSON): decode + encode de volta nos 24 vetores, 1:1 com `expect`.
    @Test
    fun ut201_jsonVectors() {
        val arr = vectors.getJSONArray("json")
        for (i in 0 until arr.length()) {
            val v = arr.getJSONObject(i)
            val name = v.getString("name")
            val message = v.getString("message")
            val jsonText = v.get("json").toString()
            val expect = v.getString("expect")
            when (message) {
                "HELLO" -> checkJson(name, expect, jsonText, ::decodeHello, ::encodeHello)
                "WELCOME" -> checkJson(name, expect, jsonText, ::decodeWelcome, ::encodeWelcome)
                "AUDIO_HELLO" -> checkJson(name, expect, jsonText, ::decodeAudioHello, ::encodeAudioHello)
                "PENDING" -> checkJson(name, expect, jsonText, ::decodePending, ::encodePending)
                "STREAM_CONFIG" -> checkJson(name, expect, jsonText, ::decodeStreamConfig, ::encodeStreamConfig)
                "SET" -> checkJson(name, expect, jsonText, ::decodeSet, ::encodeSet)
                "STATE" -> checkJson(name, expect, jsonText, ::decodeState, ::encodeState)
                "STATS" -> checkJson(name, expect, jsonText, ::decodeStats, ::encodeStats)
                "BYE" -> checkJson(name, expect, jsonText, ::decodeBye, ::encodeBye)
                else -> fail("vetor $name: mensagem desconhecida '$message'")
            }
        }
    }

    // UT-2.02: cada vetor de frames em pedaços de todos os tamanhos 1..len
    // (inclui byte a byte) → mesmos frames, erros e pendingBytes finais.
    @Test
    fun ut202_fragmentedInput() {
        val arr = vectors.getJSONArray("frames")
        for (i in 0 until arr.length()) {
            val v = arr.getJSONObject(i)
            val name = v.getString("name")
            val bytes = hex(v.getString("hex"))
            val expect = v.getJSONObject("expect")
            for (chunk in 1..bytes.size) {
                feedChunked(bytes, chunk, "frame[$name]/chunk=$chunk", expect)
            }
        }
    }

    private fun feedChunked(bytes: ByteArray, chunk: Int, ctx: String, expect: JSONObject) {
        val reader = FrameReader()
        val collected = ArrayList<Frame>()
        var error: CodecError? = null
        var pos = 0
        while (pos < bytes.size && error == null) {
            val end = minOf(pos + chunk, bytes.size)
            when (val r = reader.feed(bytes.copyOfRange(pos, end))) {
                is ReaderResult.Ok -> collected += r.frames
                is ReaderResult.Error -> error = r.error
            }
            pos = end
        }
        if (expect.has("error")) {
            assertEquals("$ctx erro", expect.getString("error"), error?.kind)
        } else {
            assertTrue("$ctx não deveria falhar: $error", error == null)
            val expected = expectFrames(expect)
            assertEquals("$ctx quantidade de frames", expected.size, collected.size)
            collected.zip(expected) { f, e -> assertFrame(f, e, ctx) }
            assertEquals("$ctx pendingBytes", expect.getInt("pendingBytes"), reader.pendingBytes)
        }
    }

    private inline fun <T> checkJson(
        name: String,
        expect: String,
        jsonText: String,
        decode: (ByteArray) -> T,
        encode: (T) -> ByteArray,
    ) {
        val ctx = "json[$name]"
        val payload = jsonText.toByteArray(Charsets.UTF_8)
        if (expect == "BadJson") {
            try {
                decode(payload)
                fail("$ctx deveria falhar com BadJson")
            } catch (e: CodecError.BadJson) {
                // esperado
            }
        } else {
            assertEquals("$ctx expect deve ser ok ou BadJson", "ok", expect)
            val decoded = decode(payload)
            val roundtrip = decode(encode(decoded))
            assertEquals("$ctx encode de volta", decoded, roundtrip)
        }
    }

    private class ExpectedFrame(
        val typeCode: Int,
        val type: String,
        val flags: Int,
        val stream: Int,
        val ptsUs: String,
        val payloadHex: String,
    )

    private fun expectFrames(expect: JSONObject): List<ExpectedFrame> {
        val arr = expect.opt("frames") as? JSONArray ?: JSONArray()
        return (0 until arr.length()).map { i ->
            val j = arr.getJSONObject(i)
            ExpectedFrame(
                typeCode = j.getInt("typeCode"),
                type = j.getString("type"),
                flags = j.getInt("flags"),
                stream = j.getInt("stream"),
                ptsUs = j.getString("ptsUs"), // string decimal (u64) nos vetores
                payloadHex = j.getString("payloadHex"),
            )
        }
    }

    private fun assertFrame(f: Frame, e: ExpectedFrame, ctx: String) {
        assertEquals("$ctx typeCode", e.typeCode, f.typeCode)
        assertEquals("$ctx type", e.type, f.type.name)
        assertEquals("$ctx flags", e.flags, f.flags)
        assertEquals("$ctx stream", e.stream, f.stream)
        assertEquals("$ctx ptsUs", e.ptsUs, f.ptsUs.toULong().toString())
        assertEquals("$ctx payloadHex", e.payloadHex, toHex(f.payload))
    }

    private fun hex(s: String): ByteArray =
        s.replace(" ", "").chunked(2).map { it.toInt(16).toByte() }.toByteArray()

    private fun toHex(b: ByteArray): String =
        b.joinToString("") { "%02x".format(it) }
}
