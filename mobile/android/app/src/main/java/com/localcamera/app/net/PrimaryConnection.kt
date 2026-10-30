package com.localcamera.app.net

import android.util.Log
import com.localcamera.app.protocol.Bye
import com.localcamera.app.protocol.Frame
import com.localcamera.app.protocol.FrameReader
import com.localcamera.app.protocol.FrameWriter
import com.localcamera.app.protocol.Hello
import com.localcamera.app.protocol.MessageType
import com.localcamera.app.protocol.encodeBye
import com.localcamera.app.store.PairingStore
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.withTimeout
import java.io.IOException
import java.net.InetSocketAddress
import java.net.Socket
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Relógio injetável (F2-T09): fonte dos timestamps (payload dos `PING`)
 * e de **todos** os esperas (PING, liveness, backoff) — nada de `sleep`
 * fixo. Em produção, [SystemClock]; nos testes, relógio virtual.
 */
interface Clock {
    /** Agora, em ms (só diferenças são usadas). */
    fun nowMs(): Long

    /** Aguarda [ms] (virtual nos testes). */
    suspend fun delay(ms: Long)
}

/** Relógio de produção: hora do sistema + `kotlinx.coroutines.delay`. */
object SystemClock : Clock {
    override fun nowMs(): Long = System.currentTimeMillis()
    override suspend fun delay(ms: Long) = kotlinx.coroutines.delay(ms)
}

/** Resultado de uma tentativa de conexão/sessão (interno ao laço de reconexão). */
internal sealed class Outcome {
    /** Falha de conexão/handshake: reconecta com backoff (sem evento `lost`). */
    data object Retry : Outcome()

    /** Sessão ativa morreu (liveness, peer fechado, falha de escrita): `lost` + backoff. */
    data object SessionEnded : Outcome()

    /** Definitivo (`revoked`, `version_mismatch`, `BYE`): para **sem** reconectar. */
    data class Terminal(val why: String) : Outcome()
}

/**
 * Socket principal do celular (F2-T09):
 *
 * - tenta [targets] **em paralelo** (2 s por host; o 1º sucesso vence,
 *   perdedores fechados);
 * - envia `HELLO` e trata `PENDING`/`WELCOME` (§6.5/6.6);
 * - `deviceKey` recebida vai para [pairingStore] — nunca para eventos/logs;
 * - mantém `PING` a cada 2 s e `PONG` para os `PING` do PC; 6 s sem `PONG`
 *   = conexão perdida → reconecta com backoff 1-2-4-8-8 s (relógio injetável);
 * - erros definitivos (`revoked`, `version_mismatch`, `BYE`) param o laço
 *   **sem** reconectar; `revoked` apaga a `deviceKey` ([ConnectionEvent.Denied]);
 * - `REQUEST_KEYFRAME` → [ConnectionEvent.KeyframeRequested].
 *
 * Escritor único: HELLO/PING/PONG/BYE e os frames do chamador passam por
 * [sendFrame] (uma lock) — a `SendQueue` (F2-T10) entra por aí em F2-T12.
 * [start] roda até erro definitivo, [close] ou cancelamento do job chamador.
 */
class PrimaryConnection(
    private val targets: List<InetSocketAddress>,
    private val pcId: String,
    private val socketFactory: (InetSocketAddress) -> Socket,
    private val clock: Clock = SystemClock,
    private val pairingStore: PairingStore? = null,
    private val onEvent: (ConnectionEvent) -> Unit,
    private val log: (String) -> Unit = { msg -> Log.i(TAG, msg) },
) {

    private val phase = SessionPhase(clock, pcId, pairingStore, onEvent, log)
    private val backoff = Backoff()
    private val writeLock = Any()
    private val starting = AtomicBoolean(false)

    @Volatile private var activeSocket: Socket? = null
    @Volatile private var closing = false

    /** Socket ativo (ou `null`): F2-T10/T12 o usa p/ `SO_SNDBUF`/NODELAY. */
    val socket: Socket? get() = activeSocket

    /**
     * Conecta (paralelo, 2 s/host) e mantém a sessão com reconexão.
     * Retorna em erro definitivo, [close] ou cancelamento. Chamada
     * repetida enquanto ativa é ignorada.
     */
    suspend fun start(hello: Hello) {
        if (!starting.compareAndSet(false, true)) return
        try {
            // Escopo próprio: filhos (conexões paralelas, leitura) terminam
            // antes que start retorne e são cancelados junto com o job.
            coroutineScope { loop(hello) }
        } finally {
            starting.set(false)
        }
    }

    /**
     * Encerra: envia `BYE{reason}` (best effort) e fecha o socket;
     * [start] retorna **sem** reconectar. Sem cancelamento do job, o
     * retorno acontece ao fim do atraso de backoff em curso.
     */
    @Synchronized
    fun close(reason: String) {
        if (closing) return
        closing = true
        activeSocket?.let { s ->
            try {
                val bytes = FrameWriter.write(Frame(MessageType.BYE.code, 0, 0, 0, encodeBye(Bye(reason))))
                synchronized(writeLock) {
                    s.getOutputStream().write(bytes)
                    s.getOutputStream().flush()
                }
            } catch (e: IOException) {
                log("BYE: falha de escrita (ignorado)")
            }
            s.closeQuietly()
        }
    }

    /**
     * Envia [frame] no socket ativo (I/O bloqueante — chamar de thread de
     * fundo). `false` se não houver socket ativo ou a escrita falhar.
     */
    fun sendFrame(frame: Frame): Boolean {
        val s = activeSocket ?: return false
        return try {
            val bytes = FrameWriter.write(frame)
            synchronized(writeLock) {
                s.getOutputStream().write(bytes)
                s.getOutputStream().flush()
            }
            true
        } catch (e: IOException) {
            log("frame ${frame.type}: falha de escrita")
            false
        }
    }

    // ------------------------------------------------------------------
    // Laço de reconexão
    // ------------------------------------------------------------------

    private suspend fun CoroutineScope.loop(hello: Hello) {
        while (!closing) {
            currentCoroutineContext().ensureActive()
            when (val outcome = attemptOnce(hello)) {
                is Outcome.Terminal -> {
                    log("definitivo: ${outcome.why}")
                    return
                }
                Outcome.SessionEnded -> {
                    if (closing) return
                    onEvent(ConnectionEvent.Lost)
                    backoff.reset() // sessão bem-sucedida → volta a 1 s
                }
                Outcome.Retry -> Unit // falha de rede: reconecta calado
            }
            currentCoroutineContext().ensureActive()
            delayBackoff(backoff.next())
        }
    }

    /** Aguarda o backoff em passos curtos para [close] ser notado logo. */
    private suspend fun delayBackoff(ms: Long) {
        var remaining = ms
        while (remaining > 0 && !closing) {
            val step = minOf(remaining, CLOSE_POLL_MS)
            clock.delay(step)
            remaining -= step
        }
    }

    private suspend fun CoroutineScope.attemptOnce(hello: Hello): Outcome {
        val socket = connectPhase() ?: return Outcome.Retry
        activeSocket = socket
        try {
            socket.tcpNoDelay = true // §3: TCP_NODELAY nos dois sentidos
            val reader = FrameReader()
            val data = Channel<ByteArray>(Channel.UNLIMITED)
            val readJob = launch(Dispatchers.IO) { phase.pumpReads(socket, data) }
            try {
                return phase.run(socket, { frame -> sendFrame(frame) }, hello, reader, data)
            } finally {
                readJob.cancel()
            }
        } finally {
            activeSocket = null
            socket.closeQuietly()
        }
    }

    /**
     * Tenta [targets] em paralelo, 2 s cada; o 1º sucesso vence (perdedores
     * fechados). `null` se todos falharem.
     *
     * O limite usa o [clock] (injetável): com `withTimeout` real, um relógio
     * virtual (testes) avançaria o prazo antes do factory — que roda em I/O
     * real — concluir, e todo host "falharia". Aqui cada passo é um
     * [CLOSE_POLL_MS] do clock.
     */
    private suspend fun connectPhase(): Socket? {
        if (targets.isEmpty()) return null
        val winners = Channel<Socket>(targets.size)
        var connected: Socket? = null
        coroutineScope {
            targets.forEach { addr ->
                launch(Dispatchers.IO) {
                    val s = tryConnect(addr)
                    if (s != null) winners.trySend(s)
                }
            }
            // Limite 2× p/ margem de um connect que termine logo após os 2 s.
            val deadline = clock.nowMs() + CONNECT_TIMEOUT_MS * 2
            while (true) {
                connected = winners.tryReceive().getOrNull()
                if (connected != null || clock.nowMs() >= deadline) break
                clock.delay(CLOSE_POLL_MS)
            }
            // Ao sair do escopo os hosts ainda em conexão são cancelados.
        }
        // Perdedores (ou vitória tardia depois do deadline): fechados.
        while (true) {
            val loser = winners.tryReceive()
            if (loser.isFailure) break // canal vazio: encerra o dreno
            loser.getOrThrow().closeQuietly()
        }
        return connected
    }

    private suspend fun tryConnect(addr: InetSocketAddress): Socket? {
        var created: Socket? = null
        return try {
            withTimeout(CONNECT_TIMEOUT_MS) {
                val s = socketFactory(addr)
                created = s
                s.connect(addr, CONNECT_TIMEOUT_MS.toInt())
                s
            }
        } catch (e: Exception) {
            created?.closeQuietly()
            log("conexão ${addr.hostString}:${addr.port}: falhou")
            null
        }
    }

    private fun Socket.closeQuietly() {
        try {
            close()
        } catch (e: IOException) {
            // já fechado
        }
    }

    companion object {
        private const val TAG = "PrimaryConn"

        /** Timeout por host na conexão paralela (2 s, F2-T09/§4). */
        const val CONNECT_TIMEOUT_MS = 2000L

        /** Intervalo dos `PING` (2 s, CHEATSHEET). */
        const val PING_INTERVAL_MS = 2000L

        /** Sem `PONG` por este tempo = conexão perdida (6 s, CHEATSHEET). */
        const val LIVENESS_TIMEOUT_MS = 6000L

        /** Passos curtos do aguardo de backoff para [close] (interno). */
        private const val CLOSE_POLL_MS = 500L

        /** `reason` que param a reconexão (F2-T09). */
        val TERMINAL_REASONS: Set<String> = setOf("revoked", "version_mismatch")
    }
}
