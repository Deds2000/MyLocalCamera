package com.localcamera.app.net

import com.localcamera.app.protocol.Frame
import com.localcamera.app.protocol.FrameWriter
import java.io.IOException
import java.io.OutputStream
import java.net.Socket
import java.util.concurrent.TimeUnit
import java.util.concurrent.locks.ReentrantLock

/**
 * Faixas de prioridade no socket principal (protocolo §11): [CTRL]
 * (`PING`/`PONG`, `SET`, `STATE`, `REQUEST_KEYFRAME`, `BYE`, ...) sempre
 * precede [VIDEO]. O áudio tem fila e socket próprios — não passa aqui.
 */
enum class Lane {
    CTRL,
    VIDEO,
}

/** Item da fila: quadro + instante (relógio injetado) em que entrou. */
private data class QueuedFrame(val frame: Frame, val enqueuedAtMs: Long)

/**
 * Fila de envio do celular (F2-T10, protocolo §11).
 *
 * - **Duas faixas, escritor único:** [enqueue] é thread-safe (encoder,
 *   eventos de controle); só [run] toca o [OutputStream]. [Lane.CTRL]
 *   sempre sai antes de [Lane.VIDEO].
 * - **Descarte por idade:** a cada `enqueue` de [Lane.VIDEO], se o quadro
 *   mais antigo da faixa tiver idade > [MAX_AGE_MS] (150 ms, relógio
 *   injetado via [clock]), descartam-se os quadros **não-chave** da cabeça
 *   até ficar ≤ [MAX_AGE_MS] — ou até chegar a um keyframe, que **nunca**
 *   é descartado. Cada `enqueue` que descartar ≥1 quadro invoca
 *   [onDropStale] **uma vez** (→ PC pede/força keyframe). Se a cabeça for
 *   um keyframe antigo, nada é descartado e o callback não dispara (o
 *   keyframe segue até ser escrito).
 * - **Socket:** aplicar [applySocketOptions] logo após criar o socket,
 *   **antes** de `connect` (`TCP_NODELAY` + `SO_SNDBUF` = [SNDBUF_BYTES]).
 *
 * [run] retorna no [stop] ou em falha de escrita (socket quebrado) — neste
 * último caso [onWriteError] é invocado uma vez e o chamador (F2-T12) decide
 * o destino da sessão (`lost` + reconexão). Assumem-se quadros conformes ao
 * protocolo (payload ≤ 8 MiB).
 */
class SendQueue(private val clock: () -> Long = System::currentTimeMillis) {

    /** Uma vez por rajada de descarte de vídeo → forçar keyframe no encoder. */
    @Volatile
    var onDropStale: (() -> Unit)? = null

    /** Uma vez, se [run] parar por falha de I/O (F2-T12: sessão perdida). */
    @Volatile
    var onWriteError: ((IOException) -> Unit)? = null

    /** Quadros de vídeo não-chave descartados desde a criação (para STATS). */
    @Volatile
    var dropped: Int = 0
        private set

    // ReentrantLock + Condition (o toolchain AGP 8.13/Kotlin 2.4 não resolve
    // `Object.wait/notify` do Kotlin — probe confirmado em F2-T10).
    private val lock = ReentrantLock()
    private val notEmpty = lock.newCondition()
    private val ctrlQueue = ArrayDeque<QueuedFrame>()
    private val videoQueue = ArrayDeque<QueuedFrame>()

    @Volatile private var stopped = false

    /**
     * Insere [frame] na [lane] com o timestamp do [clock]; thread-safe.
     * Após [stop] é no-op (o encoder não deve falhar durante o teardown).
     */
    fun enqueue(lane: Lane, frame: Frame) {
        val now = clock()
        var burstDropped = 0
        lock.lock()
        try {
            if (stopped) return
            when (lane) {
                Lane.CTRL -> ctrlQueue.addLast(QueuedFrame(frame, now))
                Lane.VIDEO -> {
                    videoQueue.addLast(QueuedFrame(frame, now))
                    burstDropped = pruneStaleVideo(now)
                }
            }
            dropped += burstDropped
            notEmpty.signal()
        } finally {
            lock.unlock()
        }
        if (burstDropped > 0) {
            onDropStale?.invoke()
        }
    }

    /**
     * Escritor único: consome as faixas (CTRL primeiro) e escreve em [out].
     * I/O bloqueante — chamar de thread de fundo (F2-T12: coroutine em
     * `Dispatchers.IO`). Retorna no [stop] ou em falha de escrita (e invoca
     * [onWriteError] uma vez).
     */
    fun run(out: OutputStream) {
        try {
            while (!stopped) {
                val item = next() ?: break
                val bytes = FrameWriter.write(item.frame)
                out.write(bytes)
                out.flush()
            }
        } catch (e: IOException) {
            stopped = true // falha de escrita: fim do escritor
            onWriteError?.invoke(e)
        }
    }

    /**
     * Idade (ms) do quadro mais antigo da faixa vídeo — p/ `STATS` da UI
     * (F2-T12). `0` se não há vídeo em fila. Somente leitura, thread-safe.
     */
    fun queueAgeMs(): Long {
        val now = clock()
        lock.lock()
        try {
            val head = videoQueue.firstOrNull() ?: return 0L
            return (now - head.enqueuedAtMs).coerceAtLeast(0L)
        } finally {
            lock.unlock()
        }
    }

    /**
     * Encerra o escritor: [run] volta em breve (mesmo ocioso) e os [enqueue]
     * seguintes viram no-op. Não drena o restante — o socket está sendo
     * fechado pelo chamador (F2-T12).
     */
    fun stop() {
        lock.lock()
        try {
            stopped = true
            notEmpty.signalAll()
        } finally {
            lock.unlock()
        }
    }

    /** Próximo item (CTRL tem prioridade) ou `null` se [stop]/fila vazia. */
    private fun next(): QueuedFrame? {
        lock.lock()
        try {
            while (!stopped) {
                val item = ctrlQueue.removeFirstOrNull() ?: videoQueue.removeFirstOrNull()
                if (item != null) return item
                notEmpty.await(IDLE_WAIT_MS, TimeUnit.MILLISECONDS) // tempo real
            }
            return null
        } finally {
            lock.unlock()
        }
    }

    /**
     * Descarta não-chaves antigos da cabeça da faixa vídeo (≤ [MAX_AGE_MS]);
     * para no 1º keyframe (nunca descartado). Devolve quantos descartou.
     * Só chamar com [lock] bloqueado.
     */
    private fun pruneStaleVideo(now: Long): Int {
        var n = 0
        while (true) {
            val head = videoQueue.firstOrNull() ?: break
            if (now - head.enqueuedAtMs <= MAX_AGE_MS) break
            if (head.frame.flags and KEYFRAME_FLAG != 0) break // nunca descarta keyframe
            videoQueue.removeFirst()
            n++
        }
        return n
    }

    companion object {
        /** Bit 0 de `flags` no cabeçalho = keyframe (CHEATSHEET §7). */
        const val KEYFRAME_FLAG: Int = 0x01

        /**
         * `SO_SNDBUF` inicial (256 KiB) — §11: buffer pequeno para a latência
         * aparecer na fila do app, não escondida no kernel.
         * **Valor pendente de medição empírica** (registra em F2-T15).
         */
        const val SNDBUF_BYTES: Int = 256 * 1024

        /** Idade máxima tolerável do quadro mais antigo da faixa vídeo. */
        const val MAX_AGE_MS: Long = 150L

        /** Espera ociosa do escritor (tempo real, interno). */
        private const val IDLE_WAIT_MS: Long = 250L

        /**
         * Opções do socket principal (§3/§11): `TCP_NODELAY` (sem Nagle) e
         * `SO_SNDBUF` reduzido para [SNDBUF_BYTES].
         *
         * `setSendBufferSize` não surte efeito **após** o `connect`: chamar
         * logo após criar o socket, antes de conectar (F2-T12).
         */
        fun applySocketOptions(socket: Socket) {
            socket.tcpNoDelay = true
            socket.sendBufferSize = SNDBUF_BYTES
        }
    }
}
