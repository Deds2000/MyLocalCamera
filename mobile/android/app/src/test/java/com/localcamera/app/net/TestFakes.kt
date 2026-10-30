package com.localcamera.app.net

import com.localcamera.app.protocol.Frame
import com.localcamera.app.protocol.FrameReader
import com.localcamera.app.protocol.FrameWriter
import com.localcamera.app.protocol.Hello
import com.localcamera.app.protocol.MessageType
import com.localcamera.app.protocol.ReaderResult
import com.localcamera.app.store.PairingStore
import com.localcamera.app.store.SecureStore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.IOException
import java.io.InputStream
import java.io.PipedInputStream
import java.io.PipedOutputStream
import java.io.OutputStream
import java.net.InetSocketAddress
import java.net.Socket
import java.net.SocketAddress
import java.util.Collections

/**
 * Fakes para [PrimaryConnectionTest] (JVM, sem dispositivo): socket em
 * memória (tubos), relógio virtual e `PairingStore` em diretório temporário.
 */

/** Socket de teste conectado a um par via [PipedInputStream]/[PipedOutputStream]. */
class FakeSocket private constructor(
    private val inStream: PipedInputStream,
    private val outStream: PipedOutputStream,
) : Socket() {

    var tcpNoDelaySet = false
    @Volatile var closedFlag = false
    @Volatile var peer: FakeSocket? = null

    override fun connect(addr: SocketAddress?, timeout: Int) {
        // no-op: o "par" já está ligado pelos tubos
    }

    override fun connect(addr: SocketAddress) = connect(addr, 0)

    override fun getInputStream(): InputStream = inStream
    override fun getOutputStream(): OutputStream = outStream
    override fun getLocalSocketAddress(): SocketAddress = InetSocketAddress("127.0.0.1", 0)
    override fun setTcpNoDelay(on: Boolean) {
        tcpNoDelaySet = on
    }

    override fun close() {
        if (closedFlag) return
        closedFlag = true
        // Semântica TCP: fechar o socket parte o tubo que alimenta a LEITURA
        // do par (o JDK não desperta o reader com close() do próprio lado —
        // só com o fim da escrita do outro lado, `closedByWriter`).
        runCatching { peer?.outStream?.close() }
        runCatching { inStream.close() }
        runCatching { outStream.close() }
    }

    override fun isClosed(): Boolean = closedFlag

    companion object {
        /** Cria um par conectado: `(cliente, servidor)`. */
        fun pair(): Pair<FakeSocket, FakeSocket> {
            val clientOut = PipedOutputStream()
            val clientIn = PipedInputStream()
            val serverOut = PipedOutputStream()
            val serverIn = PipedInputStream()
            serverIn.connect(clientOut)
            clientIn.connect(serverOut)
            val client = FakeSocket(clientIn, clientOut)
            val server = FakeSocket(serverIn, serverOut)
            client.peer = server
            server.peer = client
            return client to server
        }
    }
}

/** Escreve um quadro do lado do servidor de teste. */
fun FakeSocket.sendTestFrame(frame: Frame) {
    synchronized(this) {
        val os = getOutputStream()
        os.write(FrameWriter.write(frame))
        os.flush()
    }
}

/** "Servidor" de teste: lê quadros de [this] e entrega ao handler (I/O real). */
suspend fun FakeSocket.serve(onFrame: suspend (Frame) -> Unit) {
    val reader = FrameReader()
    val buf = ByteArray(1024 * 1024)
    while (true) {
        val n = withContext(Dispatchers.IO) {
            try {
                getInputStream().read(buf)
            } catch (e: IOException) {
                -1
            }
        }
        if (n < 0) break
        when (val r = reader.feed(buf.copyOfRange(0, n))) {
            is ReaderResult.Ok -> for (f in r.frames) onFrame(f)
            is ReaderResult.Error -> break
        }
    }
}

/** Relógio virtual: [delay] avança o tempo e cede (tempo real) a I/O paralela. */
class FakeClock(private val yieldMsReal: Long = 20) : Clock {
    @Volatile var now: Long = 0
    val delays = Collections.synchronizedList(mutableListOf<Long>())

    override fun nowMs(): Long = now

    override suspend fun delay(ms: Long) {
        require(ms > 0) { "delay deve ser > 0 (obteve $ms)" }
        delays.add(ms)
        now += ms
        if (yieldMsReal > 0) withContext(Dispatchers.IO) { Thread.sleep(yieldMsReal) }
    }
}

/** [SecureStore] em memória para testes JVM. */
class InMemorySecureStore : SecureStore {
    private val map = HashMap<String, ByteArray>()

    @Synchronized override fun put(keyId: String, secret: ByteArray) {
        map[keyId] = secret.copyOf()
    }

    @Synchronized override fun get(keyId: String): ByteArray? = map[keyId]?.copyOf()

    @Synchronized override fun delete(keyId: String) {
        map.remove(keyId)
    }
}

/** `PairingStore` real (arquivo em dir temporário) + secure store em memória. */
fun testPairingStore(): Pair<PairingStore, InMemorySecureStore> {
    val dir = java.io.File.createTempFile("pairingstore", "").apply {
        delete()
        mkdirs()
    }
    val secure = InMemorySecureStore()
    return PairingStore(dir, secure) { } to secure
}

// ---------------------------------------------------------------------------
// Construtores de quadros (payloads nos formatos do protocolo, §6/§8)
// ---------------------------------------------------------------------------

fun u64Le(v: Long): ByteArray = ByteArray(8) { i -> ((v ushr (8 * i)) and 0xFF).toByte() }

fun readU64Le(b: ByteArray, off: Int = 0): Long {
    var v = 0L
    for (i in 7 downTo 0) v = (v shl 8) or (b[off + i].toLong() and 0xFF)
    return v
}

private fun jsonFrame(type: Int, json: JSONObject): Frame =
    Frame(type, 0, 0, 0, json.toString().toByteArray(Charsets.UTF_8))

fun helloFrame(hello: Hello): Frame =
    Frame(MessageType.HELLO.code, 0, 0, 0, com.localcamera.app.protocol.encodeHello(hello))

fun pendingFrame(timeoutS: Int): Frame =
    jsonFrame(MessageType.PENDING.code, JSONObject().put("timeoutS", timeoutS))

fun welcomeFrame(sessionId: String, pcId: String = "pc-1", deviceKeyHex: String? = null): Frame =
    jsonFrame(
        MessageType.WELCOME.code,
        JSONObject().apply {
            put("accepted", true)
            put("protocol", JSONObject().put("major", 1).put("minor", 0))
            put("sessionId", sessionId)
            put("audioTicket", "aabb")
            put("pc", JSONObject().put("id", pcId).put("name", "PC").put("appVersion", "0.1.0"))
            if (deviceKeyHex != null) put("deviceKey", deviceKeyHex)
        },
    )

fun welcomeDeniedFrame(reason: String): Frame =
    jsonFrame(MessageType.WELCOME.code, JSONObject().put("accepted", false).put("reason", reason))

fun pingFrame(tsMs: Long): Frame = Frame(MessageType.PING.code, 0, 0, 0, u64Le(tsMs))

fun pongFrame(tsMs: Long): Frame = Frame(MessageType.PONG.code, 0, 0, 0, u64Le(tsMs))

fun byeFrame(reason: String): Frame =
    jsonFrame(MessageType.BYE.code, JSONObject().put("reason", reason))

/**
 * Agrupa os fakes de um teste: relógio, store, eventos, sockets e jobs do
 * servidor. [shutdown] fecha tudo para o `runTest` terminar limpo.
 */
class Rig(
    val clock: FakeClock,
    val store: PairingStore,
    val events: MutableList<ConnectionEvent>,
    val conn: PrimaryConnection,
    val clients: List<FakeSocket>,
    private val servers: List<FakeSocket>,
    private val serverJobs: List<Job>,
    val calls: () -> Int,
) {
    fun shutdown(phoneJob: Job) {
        phoneJob.cancel()
        clients.forEach { runCatching { it.close() } }
        servers.forEach { runCatching { it.close() } }
        serverJobs.forEach { it.cancel() }
    }
}

/**
 * Monta a [PrimaryConnection] com fábrica de [FakeSocket]: cada chamada do
 * `socketFactory` cria um par e lança um "servidor" de teste dirigido por
 * [script] (índice 1-based da chamada). [failConnect] faz a fábrica lançar.
 */
fun makeRig(
    targets: List<InetSocketAddress>,
    script: (Int, FakeSocket) -> suspend (Frame) -> Unit,
    failConnect: Boolean = false,
    launchServer: (suspend () -> Unit) -> Job,
): Rig {
    val clock = FakeClock()
    val (store, _) = testPairingStore()
    val events = Collections.synchronizedList(mutableListOf<ConnectionEvent>())
    val clients = Collections.synchronizedList(mutableListOf<FakeSocket>())
    val servers = Collections.synchronizedList(mutableListOf<FakeSocket>())
    val serverJobs = Collections.synchronizedList(mutableListOf<Job>())
    var calls = 0
    val conn = PrimaryConnection(
        targets = targets,
        pcId = "pc-1",
        socketFactory = {
            calls++ // conta a TENTATIVA (mesmo quando o connect falha)
            if (failConnect) throw IOException("host fora do ar")
            val (c, s) = FakeSocket.pair()
            clients.add(c)
            servers.add(s)
            val handler = script(calls, s)
            serverJobs.add(launchServer { s.serve(handler) })
            c
        },
        clock = clock,
        pairingStore = store,
        onEvent = { events.add(it) },
        log = { },
    )
    return Rig(clock, store, events, conn, clients, servers, serverJobs) { calls }
}

/** Espera (tempo real) uma condição; falha em 30 s. */
suspend fun awaitTrue(msg: String, condition: () -> Boolean) {
    val deadline = System.nanoTime() + java.util.concurrent.TimeUnit.SECONDS.toNanos(30)
    while (!condition()) {
        if (System.nanoTime() > deadline) error("timeout: $msg")
        withContext(Dispatchers.IO) { Thread.sleep(5) }
    }
}

/** Espera (tempo real) um evento que satisfaça [pred]; falha em 30 s. */
suspend fun awaitEvent(events: List<ConnectionEvent>, pred: (ConnectionEvent) -> Boolean) {
    awaitTrue("esperando evento: $events") { events.any(pred) }
}
