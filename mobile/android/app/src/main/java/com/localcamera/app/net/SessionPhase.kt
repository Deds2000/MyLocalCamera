package com.localcamera.app.net

import com.localcamera.app.protocol.Frame
import com.localcamera.app.protocol.FrameReader
import com.localcamera.app.protocol.Hello
import com.localcamera.app.protocol.MessageType
import com.localcamera.app.protocol.ReaderResult
import com.localcamera.app.protocol.Welcome
import com.localcamera.app.protocol.decodeBye
import com.localcamera.app.protocol.decodePending
import com.localcamera.app.protocol.decodeWelcome
import com.localcamera.app.protocol.encodeHello
import com.localcamera.app.store.PairingStore
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import org.json.JSONException
import org.json.JSONObject
import java.io.IOException
import java.net.Socket

/**
 * Handshake (`HELLO` → `PENDING`/`WELCOME`, §6.5/6.6) e sessão ativa
 * (`PING`/`PONG`, `BYE`, `REQUEST_KEYFRAME`) do socket principal (F2-T09).
 * Separado de [PrimaryConnection] para manter os arquivos enxutos.
 */
internal class SessionPhase(
    private val clock: Clock,
    private val pcId: String,
    private val pairingStore: PairingStore?,
    private val onEvent: (ConnectionEvent) -> Unit,
    private val log: (String) -> Unit,
) {

    /**
     * Escreve `HELLO` e mantém a conexão: handshake (`PENDING`/`WELCOME`)
     * e, se aceito, o laço de sessão (keepalive).
     *
     * [send] é o escritor único (lock de [PrimaryConnection]).
     * [data] recebe os bytes lidos de [socket] (task de I/O paralela).
     */
    suspend fun run(
        socket: Socket,
        send: (Frame) -> Boolean,
        hello: Hello,
        reader: FrameReader,
        data: Channel<ByteArray>,
    ): Outcome {
        if (!send(Frame(MessageType.HELLO.code, 0, 0, 0, encodeHello(hello)))) return Outcome.Retry
        return handshake(send, reader, data)
    }

    /**
     * Espera `PENDING`/`WELCOME`. **Sem** `PING`/liveness aqui: o PC não
     * responde `PONG` antes da sessão (aprovação pode durar 30 s, §6.2).
     */
    private suspend fun handshake(send: (Frame) -> Boolean, reader: FrameReader, data: Channel<ByteArray>): Outcome {
        for (chunk in data) {
            when (val r = reader.feed(chunk)) {
                is ReaderResult.Error -> {
                    log("codec: ${r.error.kind}; fechando")
                    return Outcome.Retry
                }
                is ReaderResult.Ok -> {
                    for (i in r.frames.indices) {
                        val f = r.frames[i]
                        when (f.type) {
                            MessageType.PENDING ->
                                onEvent(ConnectionEvent.Pending(pendingTimeout(f.payload)))
                            // Defensivo: PING antes da sessão — responde e segue esperando.
                            MessageType.PING ->
                                send(Frame(MessageType.PONG.code, 0, 0, 0, f.payload))
                            // Frames depois do WELCOME no mesmo chunk seguem p/ a sessão.
                            MessageType.WELCOME ->
                                return onWelcome(send, f, reader, data, r.frames.drop(i + 1))
                            MessageType.BYE -> return byeOutcome(f)
                            else -> Unit // fora do escopo F2-T09 antes do WELCOME
                        }
                    }
                }
            }
        }
        // Canal fechado: o peer fechou antes do WELCOME → reconectar.
        return Outcome.Retry
    }

    private suspend fun onWelcome(
        send: (Frame) -> Boolean,
        f: Frame,
        reader: FrameReader,
        data: Channel<ByteArray>,
        carry: List<Frame>,
    ): Outcome {
        val w: Welcome = runCatching { decodeWelcome(f.payload) }.getOrNull() ?: return Outcome.Retry
        if (!w.accepted) {
            val reason = w.reason ?: "protocol_error"
            onEvent(ConnectionEvent.Denied(reason))
            return if (reason in PrimaryConnection.TERMINAL_REASONS) {
                pairingStore?.deleteDeviceKey(pcId) // §6.3: pede novo pareamento por QR
                Outcome.Terminal("denied:$reason")
            } else {
                Outcome.Retry // ex.: busy → reconectar com backoff
            }
        }
        val sessionId = w.sessionId ?: return Outcome.Retry
        val pc = w.pc ?: return Outcome.Retry
        onEvent(ConnectionEvent.Welcome(sessionId, pc))
        saveDeviceKey(f.payload, pc.id)
        return session(send, reader, data, carry)
    }

    /** `deviceKey` (64 hex = 32 bytes) só no pareamento aprovado com "Permitir sempre". */
    private fun saveDeviceKey(payload: ByteArray, pcId: String) {
        val store = pairingStore ?: return
        val key = extractDeviceKey(payload) ?: return
        try {
            store.saveDeviceKey(pcId, key)
        } catch (e: Exception) {
            log("deviceKey: não salva (formato)") // a chave nunca vai a log
        }
    }

    /**
     * Lê `deviceKey` do JSON do WELCOME. O modelo do F2-T04 não expõe o
     * campo (segredo): lemos direto do payload. `null` se ausente/inválido.
     */
    private fun extractDeviceKey(payload: ByteArray): ByteArray? {
        val hex: String = try {
            JSONObject(payload.toString(Charsets.UTF_8)).optString("deviceKey")
        } catch (e: JSONException) {
            return null
        }
        if (hex.length != DEVICE_KEY_HEX_LEN) return null
        return try {
            ByteArray(DEVICE_KEY_BYTES) { i -> (hex[2 * i].hexVal() * 16 + hex[2 * i + 1].hexVal()).toByte() }
        } catch (e: IllegalArgumentException) {
            null
        }
    }

    private fun byeOutcome(f: Frame): Outcome {
        val reason = runCatching { decodeBye(f.payload).reason }.getOrNull() ?: "protocol_error"
        onEvent(ConnectionEvent.Bye(reason))
        return Outcome.Terminal("bye:$reason")
    }

    /**
     * Sessão ativa: `PING` a cada 2 s (payload u64 ms); `PONG` para o
     * `PING` do PC (mesmo u64); **6 s sem `PONG` = conexão perdida**
     * (CHEATSHEET). `BYE` é definitivo. `REQUEST_KEYFRAME` → evento.
     */
    private suspend fun session(
        send: (Frame) -> Boolean,
        reader: FrameReader,
        data: Channel<ByteArray>,
        carry: List<Frame>,
    ): Outcome {
        val t0 = clock.nowMs()
        var lastPong = t0
        var nextPing = t0 + PrimaryConnection.PING_INTERVAL_MS
        var pending: List<Frame> = carry // frames do mesmo chunk do WELCOME, se houve
        while (true) {
            currentCoroutineContext().ensureActive()
            // Drena: frames pendentes + tudo que já está na fila (PONG renova o liveness).
            drain@ while (true) {
                if (pending.isEmpty()) {
                    val chunk = data.tryReceive().getOrNull() ?: break@drain
                    when (val r = reader.feed(chunk)) {
                        is ReaderResult.Error -> {
                            log("codec: ${r.error.kind}; fechando")
                            return Outcome.SessionEnded
                        }
                        is ReaderResult.Ok -> pending = r.frames
                    }
                }
                val f = pending.first()
                pending = pending.drop(1)
                when (f.type) {
                    MessageType.PONG -> lastPong = clock.nowMs()
                    MessageType.PING ->
                        if (!send(Frame(MessageType.PONG.code, 0, 0, 0, f.payload))) {
                            return Outcome.SessionEnded
                        }
                    MessageType.REQUEST_KEYFRAME ->
                        onEvent(ConnectionEvent.KeyframeRequested)
                    MessageType.BYE -> return byeOutcome(f)
                    // F2-T12: o StateReducer aplica e responde com STATE (§9).
                    MessageType.SET ->
                        onEvent(ConnectionEvent.Set(f.payload.toString(Charsets.UTF_8)))
                    // STATS/UNKNOWN: não são processados aqui (F2-T12/logs).
                    else -> Unit
                }
            }
            if (data.isClosedForReceive) return Outcome.SessionEnded
            val now = clock.nowMs()
            if (now - lastPong >= PrimaryConnection.LIVENESS_TIMEOUT_MS) return Outcome.SessionEnded
            if (now >= nextPing) {
                if (!send(Frame(MessageType.PING.code, 0, 0, 0, u64Le(now)))) return Outcome.SessionEnded
                nextPing = now + PrimaryConnection.PING_INTERVAL_MS // evita rajada após atraso
            }
            // Aguarda o próximo prazo (PING ou liveness) com relógio injetável.
            val wait = minOf(nextPing - now, PrimaryConnection.LIVENESS_TIMEOUT_MS - (now - lastPong))
            if (wait > 0) clock.delay(wait)
        }
    }

    /** Tarefa de I/O: leitura bloqueante do socket → [data]; fecha o canal no fim. */
    suspend fun pumpReads(socket: Socket, data: Channel<ByteArray>) {
        val buf = ByteArray(READ_BUF_BYTES)
        try {
            val input = socket.getInputStream()
            while (true) {
                val n = input.read(buf)
                if (n < 0) break
                data.send(buf.copyOfRange(0, n))
            }
        } catch (e: IOException) {
            // socket fechado (close()/peer): fim das leituras
        } finally {
            data.close()
        }
    }

    private fun pendingTimeout(payload: ByteArray): Int =
        runCatching { decodePending(payload).timeoutS }.getOrNull() ?: 0

    private fun u64Le(ms: Long): ByteArray =
        ByteArray(8) { i -> ((ms ushr (8 * i)) and 0xFF).toByte() }

    private fun Char.hexVal(): Int = when (this) {
        in '0'..'9' -> this - '0'
        in 'a'..'f' -> this - 'a' + 10
        in 'A'..'F' -> this - 'A' + 10
        else -> throw IllegalArgumentException("hex: '$this'")
    }

    private companion object {
        const val DEVICE_KEY_HEX_LEN = 64
        const val DEVICE_KEY_BYTES = 32
        const val READ_BUF_BYTES = 64 * 1024
    }
}
