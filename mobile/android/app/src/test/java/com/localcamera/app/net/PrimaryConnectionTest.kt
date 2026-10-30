package com.localcamera.app.net

import com.localcamera.app.protocol.AppInfo
import com.localcamera.app.protocol.Auth
import com.localcamera.app.protocol.Caps
import com.localcamera.app.protocol.DeviceInfo
import com.localcamera.app.protocol.Frame
import com.localcamera.app.protocol.Hello
import com.localcamera.app.protocol.MessageType
import com.localcamera.app.protocol.PcInfo
import com.localcamera.app.protocol.ProtocolVersion
import com.localcamera.app.protocol.decodeBye
import com.localcamera.app.protocol.decodeHello
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.net.InetSocketAddress

/**
 * UT-2.09 (F2-T09) — PrimaryConnection com socket fake (loopback em memória)
 * e relógio virtual: handshake (WELCOME aceito/recusado, PENDING), keepalive
 * (PING/PONG, liveness 6 s), BYE, backoff e erros definitivos.
 *
 * Toda chamada a [Rig.shutdown] está em `finally`: um `assert` que falhe no
 * meio do teste ainda fecha os tubos/cancela os jobs e o `runTest` termina.
 */
class PrimaryConnectionTest {

    private val addr1 = InetSocketAddress("10.0.0.1", 48650)
    private val addr2 = InetSocketAddress("10.0.0.2", 48650)
    private val pc1 = "pc-1"
    private val deviceKeyHex = "ab".repeat(32) // 256 bits
    private val token = "00112233445566778899aabbccddeeff"

    private val hello = Hello(
        protocol = ProtocolVersion(1, 0),
        app = AppInfo("1.0", "android"),
        device = DeviceInfo("11111111-1111-4111-8111-111111111111", "Pixel", "Pixel 8", 35),
        auth = Auth.Pair(token),
        caps = Caps(codecs = listOf("h264"), audio = true),
    )

    // WELCOME aceito: salva `deviceKey`, confirma HELLO, responde PING com PONG,
    // e sessão cai quando o peer fecha (→ `lost`).
    @Test
    fun ut209_welcomeAccepted_savesDeviceKeyAndKeepsAlive() = runTest {
        val pongs = java.util.Collections.synchronizedList(mutableListOf<Long>())
        val rig = makeRig(listOf(addr1), { idx, s ->
            if (idx != 1) return@makeRig { }
            { frame ->
                when (frame.type) {
                    MessageType.HELLO -> {
                        val h = decodeHello(frame.payload)
                        val pair = h.auth as? Auth.Pair ?: error("auth deveria ser pair")
                        assertEquals(token, pair.token)
                        s.sendTestFrame(welcomeFrame("s1", pc1, deviceKeyHex))
                        s.sendTestFrame(pingFrame(42L))
                    }
                    MessageType.PONG -> {
                        pongs.add(readU64Le(frame.payload))
                        s.close() // fim de sessão → lost
                    }
                    else -> Unit
                }
            }
        }, launchServer = { launch { it() } })
        val job = launch { rig.conn.start(hello) }
        try {
            awaitEvent(rig.events) { it is ConnectionEvent.Welcome }
            val w = rig.events.filterIsInstance<ConnectionEvent.Welcome>().first()
            assertEquals("s1", w.sessionId)
            assertEquals(pc1, w.pc.id)
            val key = rig.store.deviceKey(pc1)
            assertNotNull("deviceKey recebida deve ser salva no store", key)
            assertArrayEquals(ByteArray(32) { 0xAB.toByte() }, key)
            assertTrue("TCP_NODELAY deve estar ativo (§3)", rig.clients[0].tcpNoDelaySet)

            awaitEvent(rig.events) { it is ConnectionEvent.Lost }
            assertEquals(listOf(42L), pongs) // PONG ecoou o timestamp do PING
        } finally {
            rig.shutdown(job)
        }
    }

    // PENDING antes do WELCOME: evento `pending(30)` e depois `welcome`.
    @Test
    fun ut209_pendingEntaoWelcome() = runTest {
        val rig = makeRig(listOf(addr1), { idx, s ->
            if (idx != 1) return@makeRig { }
            { frame ->
                if (frame.type == MessageType.HELLO) {
                    s.sendTestFrame(pendingFrame(30))
                    s.sendTestFrame(welcomeFrame("s2", pc1))
                    s.close()
                }
            }
        }, launchServer = { launch { it() } })
        val job = launch { rig.conn.start(hello) }
        try {
            awaitEvent(rig.events) { it is ConnectionEvent.Welcome }
            assertEquals(
                listOf(
                    ConnectionEvent.Pending(30),
                    ConnectionEvent.Welcome("s2", PcInfo(pc1, "PC", "0.1.0")),
                ),
                rig.events.take(2),
            )
        } finally {
            rig.shutdown(job)
        }
    }

    // `revoked`: definitivo — apaga a `deviceKey`, não reconecta, start() retorna.
    @Test
    fun ut209_revokedApagaChaveENaoReconecta() = runTest {
        val rig = makeRig(listOf(addr1), { idx, s ->
            if (idx != 1) return@makeRig { }
            { frame ->
                if (frame.type == MessageType.HELLO) {
                    s.sendTestFrame(welcomeDeniedFrame("revoked"))
                    s.close()
                }
            }
        }, launchServer = { launch { it() } })
        rig.store.saveDeviceKey(pc1, ByteArray(32) { 0xAB.toByte() })

        val job = launch { rig.conn.start(hello) }
        try {
            awaitTrue("start() deveria retornar (erro definitivo)") { job.isCompleted }

            assertEquals(listOf(ConnectionEvent.Denied("revoked")), rig.events.toList())
            assertNull("revoked deve apagar a deviceKey", rig.store.deviceKey(pc1))
            assertEquals(1, rig.calls()) // nenhuma reconexão
            assertTrue(rig.clients[0].closedFlag)
        } finally {
            rig.shutdown(job)
        }
    }

    // `busy` (não definitivo): `denied` → backoff 1 s → reconecta → `welcome`.
    @Test
    fun ut209_busyReconectaComBackoffEWelcome() = runTest {
        val rig = makeRig(listOf(addr1), { idx, s ->
            when (idx) {
                1 -> { frame: Frame ->
                    if (frame.type == MessageType.HELLO) {
                        s.sendTestFrame(welcomeDeniedFrame("busy"))
                        s.close()
                    }
                }
                2 -> { frame: Frame ->
                    if (frame.type == MessageType.HELLO) {
                        s.sendTestFrame(welcomeFrame("s3", pc1))
                        s.close()
                    }
                }
                else -> { _: Frame -> }
            }
        }, launchServer = { launch { it() } })
        val job = launch { rig.conn.start(hello) }
        try {
            awaitEvent(rig.events) { it is ConnectionEvent.Welcome }
            assertEquals(ConnectionEvent.Denied("busy"), rig.events.first())
            assertEquals("s3", rig.events.filterIsInstance<ConnectionEvent.Welcome>().first().sessionId)
            assertTrue("2ª tentativa abre novo socket", rig.calls() >= 2)
            // Passos de 500 ms: connectPhase e backoff (1 s) usam o mesmo clock.
            assertEquals(listOf(500L, 500L), rig.clock.delays.take(2))
        } finally {
            rig.shutdown(job)
        }
    }

    // Sessão ativa: PING a cada 2 s; sem PONG por 6 s → `lost` → backoff
    // (reset p/ 1 s) → nova conexão; `BYE` da 2ª → definitivo, start() retorna.
    @Test
    fun ut209_pongPerdidoLostReconectaEByeDefinitivo() = runTest {
        val rig = makeRig(listOf(addr1), { idx, s ->
            when (idx) {
                1 -> { frame: Frame ->
                    if (frame.type == MessageType.HELLO) {
                        s.sendTestFrame(welcomeFrame("s4", pc1))
                        s.sendTestFrame(pingFrame(7L)) // o PC também pinga
                        // mas nunca responde PONG aos nossos PINGs
                    }
                }
                2 -> { frame: Frame ->
                    if (frame.type == MessageType.HELLO) {
                        s.sendTestFrame(byeFrame("busy"))
                        s.close()
                    }
                }
                else -> { _: Frame -> }
            }
        }, launchServer = { launch { it() } })
        val job = launch { rig.conn.start(hello) }
        try {
            awaitTrue("start() deveria retornar (BYE é definitivo)") { job.isCompleted }

            assertEquals(
                listOf(
                    ConnectionEvent.Welcome("s4", PcInfo(pc1, "PC", "0.1.0")),
                    ConnectionEvent.Lost,
                    ConnectionEvent.Bye("busy"),
                ),
                rig.events.toList(),
            )
            assertEquals(2, rig.calls())
            // Determinístico: exatamente 3 PINGs a 2 s (liveness 6 s) e o
            // backoff de 1 s (2×500) após sessão bem-sucedida. O connectPhase
            // também consome passos de 500 ms, por isso não soma o total.
            assertEquals(6000L, rig.clock.delays.filter { it >= 2000L }.sum())
            assertTrue("backoff de 1 s após lost", rig.clock.delays.count { it == 500L } >= 2)
        } finally {
            rig.shutdown(job)
        }
    }

    // `BYE` na sessão: evento `bye(reason)` e encerramento definitivo.
    @Test
    fun ut209_byeDefinitivoSemReconectar() = runTest {
        val rig = makeRig(listOf(addr1), { idx, s ->
            if (idx != 1) return@makeRig { }
            { frame ->
                if (frame.type == MessageType.HELLO) {
                    s.sendTestFrame(welcomeFrame("s5", pc1))
                    s.sendTestFrame(byeFrame("busy"))
                }
            }
        }, launchServer = { launch { it() } })
        val job = launch { rig.conn.start(hello) }
        try {
            awaitTrue("start() deveria retornar (BYE é definitivo)") { job.isCompleted }

            assertEquals(
                listOf(
                    ConnectionEvent.Welcome("s5", PcInfo(pc1, "PC", "0.1.0")),
                    ConnectionEvent.Bye("busy"),
                ),
                rig.events.toList(),
            )
            assertEquals(1, rig.calls()) // sem reconexão
        } finally {
            rig.shutdown(job)
        }
    }

    // Dois hosts em paralelo: o 1º sucesso vence; o perdedor é fechado e
    // recebe `HELLO` de ninguém.
    @Test
    fun ut209_paraleloPrimeiroSucessoVence() = runTest {
        val helloSeen = java.util.Collections.synchronizedList(mutableListOf(false, false))
        val rig = makeRig(listOf(addr1, addr2), { idx, s ->
            { frame ->
                if (frame.type == MessageType.HELLO && idx <= 2) {
                    helloSeen[idx - 1] = true
                    s.sendTestFrame(welcomeFrame("sA", pc1))
                    s.close()
                }
            }
        }, launchServer = { launch { it() } })
        val job = launch { rig.conn.start(hello) }
        try {
            awaitEvent(rig.events) { it is ConnectionEvent.Welcome }
            assertEquals(1, helloSeen.count { it }) // só o vencedor recebeu HELLO
            assertEquals(1, rig.events.filterIsInstance<ConnectionEvent.Welcome>().size)
            val winnerIdx = helloSeen.indexOfFirst { it }
            assertNotEquals(-1, winnerIdx)
            assertTrue("perdedor deve ser fechado", rig.clients[1 - winnerIdx].closedFlag)
        } finally {
            rig.shutdown(job)
        }
    }

    // Todos os hosts falham: connectPhase esgota 4 s (8×500) por tentativa;
    // backoff 1 s, 2 s (passos de 500 ms no clock virtual).
    @Test
    fun ut209_todosFalhamBackoff12() = runTest {
        val rig = makeRig(
            listOf(addr1),
            script = { _, _ -> { } },
            failConnect = true,
            launchServer = { launch { it() } },
        )
        val job = launch { rig.conn.start(hello) }
        try {
            awaitTrue("3ª tentativa de conexão") { rig.calls() >= 3 }
            assertEquals(
                List(22) { 500L }, // 8 (conexão 1) + 2 (1 s) + 8 (conexão 2) + 4 (2 s)
                rig.clock.delays.take(22),
            )
            assertEquals(0, rig.events.size) // falha de rede não gera eventos
        } finally {
            rig.shutdown(job)
        }
    }

    // close(): envia `BYE{reason}` ao PC, fecha o socket, start() retorna
    // sem `lost` (encerramento intencional).
    @Test
    fun ut209_closeEnviaByeEPara() = runTest {
        var byeReason: String? = null
        val rig = makeRig(listOf(addr1), { idx, s ->
            if (idx != 1) return@makeRig { }
            { frame ->
                when (frame.type) {
                    MessageType.HELLO -> s.sendTestFrame(welcomeFrame("s6", pc1))
                    MessageType.BYE -> {
                        byeReason = decodeBye(frame.payload).reason
                        s.close()
                    }
                    else -> Unit
                }
            }
        }, launchServer = { launch { it() } })
        val job = launch { rig.conn.start(hello) }
        try {
            awaitEvent(rig.events) { it is ConnectionEvent.Welcome }
            rig.conn.close("user_closed")
            awaitTrue("start() deveria retornar após close()") { job.isCompleted }

            assertEquals("user_closed", byeReason)
            assertEquals(
                listOf(ConnectionEvent.Welcome("s6", PcInfo(pc1, "PC", "0.1.0"))),
                rig.events.toList(), // sem `lost` em close intencional
            )
        } finally {
            rig.shutdown(job)
        }
    }
}
