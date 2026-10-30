package com.localcamera.app.net

import com.localcamera.app.protocol.Frame
import com.localcamera.app.protocol.FrameReader
import com.localcamera.app.protocol.MessageType
import com.localcamera.app.protocol.ReaderResult
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.io.OutputStream
import java.net.Socket
import java.util.Collections

/**
 * UT-2.04 (F2-T10) — [SendQueue]: controle passa na frente do vídeo;
 * descarte por tempo (>150 ms, relógio virtual); keyframe mais recente
 * preservado; pedido de keyframe uma vez por rajada; `TCP_NODELAY` +
 * `SO_SNDBUF` documentados. JVM, sem dispositivo.
 */
class SendQueueTest {

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /** `VIDEO` de teste: [ptsUs] no cabeçalho, `stream=1`, flag de keyframe opcional. */
    private fun videoFrame(ptsUs: Long, keyframe: Boolean = false): Frame =
        Frame(
            typeCode = MessageType.VIDEO.code,
            flags = if (keyframe) SendQueue.KEYFRAME_FLAG else 0,
            stream = 1,
            ptsUs = ptsUs,
            payload = byteArrayOf(0x01),
        )

    private fun pingFrame(): Frame =
        Frame(MessageType.PING.code, 0, 0, 0, u64Le(0L))

    private fun label(f: Frame): String = when (f.type) {
        MessageType.VIDEO -> if (f.flags and SendQueue.KEYFRAME_FLAG != 0) "K${f.ptsUs}" else "v${f.ptsUs}"
        else -> f.type.name
    }

    /** `OutputStream` que grava os quadros completos (bytes via [FrameReader]). */
    private class RecordingOut : ByteArrayOutputStream() {
        private val reader = FrameReader()
        val frames = Collections.synchronizedList(mutableListOf<Frame>())

        override fun write(b: ByteArray, off: Int, len: Int) {
            super.write(b, off, len)
            when (val r = reader.feed(b.copyOfRange(off, off + len))) {
                is ReaderResult.Ok -> frames.addAll(r.frames)
                is ReaderResult.Error -> throw IOException("frame inválido no teste: ${r.error}")
            }
        }
    }

    /** `OutputStream` que quebra na 1ª escrita (simula socket partido). */
    private class FailingOut : OutputStream() {
        var writes = 0

        override fun write(b: Int) {
            writes++
            throw IOException("pipe quebrado (teste)")
        }

        override fun write(b: ByteArray, off: Int, len: Int) {
            writes++
            throw IOException("pipe quebrado (teste)")
        }
    }

    private fun startWriter(q: SendQueue, out: OutputStream): Thread {
        val t = Thread({ q.run(out) }, "sendqueue-test-writer")
        t.isDaemon = true
        t.start()
        return t
    }

    // ------------------------------------------------------------------
    // UT-2.04
    // ------------------------------------------------------------------

    /** Vídeo enfileirado antes do controle sai depois dele (prioridade de faixa). */
    @Test
    fun ut204_controlAlwaysPrecedesVideo() {
        val q = SendQueue { 0L }
        val out = RecordingOut()
        q.enqueue(Lane.VIDEO, videoFrame(1))
        q.enqueue(Lane.VIDEO, videoFrame(2))
        q.enqueue(Lane.CTRL, pingFrame()) // entra DEPOIS dos dois vídeos
        q.enqueue(Lane.VIDEO, videoFrame(3))
        val t = startWriter(q, out)
        try {
            runBlocking { awaitTrue("4 frames escritos") { out.frames.size >= 4 } }
        } finally {
            q.stop()
            t.join(5000)
        }
        assertFalse("escritor deve terminar após stop()", t.isAlive)
        assertEquals(listOf("PING", "v1", "v2", "v3"), out.frames.map { label(it) })
    }

    /**
     * Descarte por idade com relógio virtual: não-chaves >150 ms saem da
     * cabeça; keyframe nunca é descartado (mesmo com 200+ ms de idade);
     * `onDropStale` dispara 1× por rajada (rajada de 2 → 1 chamada);
     * cabeça keyframe antiga → nada descartado, sem callback.
     */
    @Test
    fun ut204_timeBasedDiscardKeepsKeyframeAndRequestsOncePerBurst() {
        var now = 0L
        val q = SendQueue { now }
        val calls = Collections.synchronizedList(mutableListOf<Long>())
        q.onDropStale = { calls.add(now) }

        now = 0;   q.enqueue(Lane.VIDEO, videoFrame(0))            // v0
        now = 40;  q.enqueue(Lane.VIDEO, videoFrame(40))           // v40
        now = 80;  q.enqueue(Lane.VIDEO, videoFrame(80))           // v80
        now = 160; q.enqueue(Lane.VIDEO, videoFrame(160))          // v0 age 160 → descarta
        assertEquals("1 descarte", 1, q.dropped)
        assertEquals(listOf(160L), calls)

        now = 240; q.enqueue(Lane.VIDEO, videoFrame(240))          // rajada: v40+v80
        assertEquals("3 descartes", 3, q.dropped)
        assertEquals("rajada de 2 → 1 callback", listOf(160L, 240L), calls)

        now = 280; q.enqueue(Lane.VIDEO, videoFrame(280, keyframe = true)) // K280
        assertEquals("sem descarte", 3, q.dropped)

        now = 400; q.enqueue(Lane.VIDEO, videoFrame(400))          // v160+v240; para em K280
        assertEquals("5 descartes", 5, q.dropped)
        assertEquals(listOf(160L, 240L, 400L), calls)

        now = 480; q.enqueue(Lane.VIDEO, videoFrame(480))          // cabeça = K280 (200 ms): não descarta
        assertEquals("keyframe antigo bloqueia descarte", 5, q.dropped)
        assertEquals("sem descarte → sem callback", listOf(160L, 240L, 400L), calls)

        val out = RecordingOut()
        val t = startWriter(q, out)
        try {
            runBlocking { awaitTrue("3 frames restantes") { out.frames.size >= 3 } }
        } finally {
            q.stop()
            t.join(5000)
        }
        // K280 preservado na cabeça mesmo >150 ms de idade na hora do descarte.
        assertEquals(listOf("K280", "v400", "v480"), out.frames.map { label(it) })
        assertTrue("keyframe preservado deve manter a flag",
            out.frames.first().flags and SendQueue.KEYFRAME_FLAG != 0)
    }

    /** [SendQueue.stop] encerra o escritor (mesmo ocioso); enqueue depois = no-op. */
    @Test
    fun ut204_stopEndsWriterAndIgnoresLateEnqueue() {
        val q = SendQueue { 0L }
        val out = RecordingOut()
        val t = startWriter(q, out)
        q.enqueue(Lane.VIDEO, videoFrame(0))
        runBlocking { awaitTrue("1 frame escrito") { out.frames.size >= 1 } }
        q.stop()
        t.join(5000)
        assertFalse("escritor deve terminar após stop()", t.isAlive)
        q.enqueue(Lane.VIDEO, videoFrame(1))
        q.enqueue(Lane.CTRL, pingFrame())
        assertEquals("no-op após stop", 1, out.frames.size)
        assertEquals(0, q.dropped)
    }

    /** Falha de escrita: escritor para, [SendQueue.onWriteError] 1×, enqueues viram no-op. */
    @Test
    fun ut204_writeErrorStopsWriterAndNotifiesOnce() {
        val q = SendQueue { 0L }
        val out = FailingOut()
        var errors = 0
        q.onWriteError = { errors++ }
        val t = startWriter(q, out)
        q.enqueue(Lane.CTRL, pingFrame())
        runBlocking { awaitTrue("erro de escrita notificado") { errors >= 1 } }
        t.join(5000)
        assertFalse(t.isAlive)
        q.enqueue(Lane.CTRL, pingFrame()) // no-op: writer já parou
        assertEquals("onWriteError uma vez", 1, errors)
        assertEquals("quebrou na 1ª escrita", 1, out.writes)
    }

    /** `TCP_NODELAY` + `SO_SNDBUF = SNDBUF_BYTES` (256 KiB, documentado). */
    @Test
    fun ut204_socketOptionsApplyNodelayAndSndbuf() {
        val s = Socket()
        try {
            SendQueue.applySocketOptions(s)
            assertTrue("TCP_NODELAY deve ficar ativo", s.tcpNoDelay)
            assertEquals("SO_SNDBUF = 256 KiB", SendQueue.SNDBUF_BYTES, s.sendBufferSize)
            assertEquals(256 * 1024, SendQueue.SNDBUF_BYTES)
        } finally {
            s.close()
        }
    }
}
