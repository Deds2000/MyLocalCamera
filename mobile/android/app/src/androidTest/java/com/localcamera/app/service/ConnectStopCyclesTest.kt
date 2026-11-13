package com.localcamera.app.service

import android.Manifest
import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.Build
import android.os.Debug
import android.util.Log
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import com.localcamera.app.protocol.Auth
import com.localcamera.app.protocol.Frame
import com.localcamera.app.protocol.FrameReader
import com.localcamera.app.protocol.FrameWriter
import com.localcamera.app.protocol.MessageType
import com.localcamera.app.protocol.PcInfo
import com.localcamera.app.protocol.ProtocolVersion
import com.localcamera.app.protocol.ReaderResult
import com.localcamera.app.protocol.Welcome
import com.localcamera.app.protocol.decodeHello
import com.localcamera.app.protocol.encodeWelcome
import com.localcamera.app.store.PairingStore
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.IOException
import java.io.OutputStream
import java.net.Inet4Address
import java.net.ServerSocket
import java.net.Socket
import java.util.UUID
import java.util.concurrent.CountDownLatch
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

/** LT-2.02: 50 ciclos contra PC fake no IPv4 Wi-Fi do aparelho.
 * Valida HELLO/token fictício; WELCOME não inclui deviceKey; PING/PONG tratado.
 * Cada ciclo espera STATE/STREAM_CONFIG, mantém 300 ms, para e aguarda EOF.
 * PSS no início, a cada 10 ciclos e no fim (crescimento máximo: 40 MB).
 * URI/segredos não são logados; finally para serviço, esquece PC e fecha thread.
 */
@RunWith(AndroidJUnit4::class)
class ConnectStopCyclesTest {

    @get:Rule
    val grantCamera = GrantPermissionRule.grant(Manifest.permission.CAMERA)

    // Mesmo padrão do ServiceSmokeTest: Android 13+ a notificação do FGS
    // exige POST_NOTIFICATIONS; em API < 33 a regra reusa CAMERA (idempotente).
    @get:Rule
    val grantNotifications: GrantPermissionRule =
        if (Build.VERSION.SDK_INT >= 33) {
            GrantPermissionRule.grant(Manifest.permission.POST_NOTIFICATIONS)
        } else {
            GrantPermissionRule.grant(Manifest.permission.CAMERA)
        }

    private class Cycle {
        /** App processou o WELCOME (STATE/STREAM_CONFIG chegaram ao fake). */
        val established = CountDownLatch(1)

        /** Socket do ciclo encerrou (BYE ou EOF). */
        val closed = CountDownLatch(1)
    }

    @Volatile private var serverError: String? = null

    @Test(timeout = TOTAL_TIMEOUT_MS)
    fun fifty_connect_stop_cycles_no_crash_and_pss_stable() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext.applicationContext
        // O teste roda sem activity: simula app visível (guarda da armadilha §6),
        // como no ServiceSmokeTest.
        val fakePcAddress = wifiIpv4Address(context)
        val server = ServerSocket(0, 1, fakePcAddress)
        val cycles = LinkedBlockingQueue<Cycle>()
        AppForeground.onActivityResumed()
        val thread = Thread({ serverLoop(server, CYCLES, cycles) }, FAKE_PC_THREAD)
        thread.start()
        var pssStart = 0L
        try {
            // Defensivo: sem sessão ativa antes do primeiro ciclo.
            StreamingService.stopStreaming(context)
            Thread.sleep(SETTLE_MS)
            // Se uma execução anterior caiu antes do finally, o PC fake pode ter
            // ficado persistido e o HELLO usaria deviceKey em vez do token da URI.
            PairingStore(context).forgetPc(PC_ID)
            pssStart = Debug.getPss()
            Log.i(TAG, "cycle=0 pssKb=$pssStart")
            // URI montada em memória; nunca registrada em log.
            val uri = "localcamera://connect?v=1&hosts=${fakePcAddress.hostAddress}&port=${server.localPort}" +
                "&token=$TOKEN&pc=$PC_ID&name=$PC_NAME"
            for (cycle in 1..CYCLES) {
                val st = Cycle()
                cycles.put(st)
                runCycle(context, uri, cycle, st)
                if (cycle % PSS_SAMPLE_EVERY == 0) {
                    val pss = Debug.getPss()
                    Log.i(TAG, "cycle=$cycle pssKb=$pss")
                }
            }
            Thread.sleep(SETTLE_MS)
            val pssEnd = Debug.getPss()
            val delta = pssEnd - pssStart
            Log.i(TAG, "cycles=$CYCLES pssStartKb=$pssStart pssEndKb=$pssEnd deltaKb=$delta")
            assertTrue(
                "PSS final cresceu $delta KB (limite ${PSS_MAX_GROWTH_KB} KB): possível vazamento",
                delta <= PSS_MAX_GROWTH_KB,
            )
        } finally {
            // Nunca deixar serviço/pareamento ativo nem thread do fake viva.
            StreamingService.stopStreaming(context)
            try {
                PairingStore(context).forgetPc(PC_ID)
            } catch (e: Exception) {
                Log.w(TAG, "forgetPc falhou (${e.javaClass.simpleName})")
            }
            AppForeground.onActivityPaused()
            try {
                server.close()
            } catch (_: IOException) {
                // já fechado
            }
            thread.interrupt()
            thread.join(JOIN_TIMEOUT_MS)
            assertFalse("thread do PC fake não terminou (vazou)", thread.isAlive)
        }
    }

    /** Um ciclo: start → WELCOME → segurar → stop → socket fechado → settle. */
    private fun runCycle(context: Context, uri: String, cycle: Int, st: Cycle) {
        StreamingService.startStreaming(context, uri)
        val established = st.established.await(WELCOME_WAIT_MS, TimeUnit.MILLISECONDS)
        assertTrue("WELCOME não processado pelo app no ciclo $cycle", established)
        Thread.sleep(HOLD_MS)
        StreamingService.stopStreaming(context)
        val closed = st.closed.await(CLOSE_WAIT_MS, TimeUnit.MILLISECONDS)
        assertTrue("socket do ciclo $cycle não encerrou (BYE/EOF)", closed)
        val err = serverError
        if (err != null) {
            fail("protocolo: $err (ciclo $cycle)")
        }
        Thread.sleep(SETTLE_MS)
    }

    /** Thread única: 1 conexão por ciclo; sai quando o server socket fecha. */
    private fun serverLoop(server: ServerSocket, cycles: Int, cycleQueue: LinkedBlockingQueue<Cycle>) {
        val buf = ByteArray(READ_BUF_BYTES)
        for (i in 1..cycles) {
            val cycle = try {
                cycleQueue.take()
            } catch (_: InterruptedException) {
                Thread.currentThread().interrupt()
                return
            }
            val socket = try {
                server.accept()
            } catch (e: IOException) {
                return // finally do teste fechou o ServerSocket
            }
            try {
                handleConnection(socket, buf, i, cycle)
            } catch (e: Exception) {
                markError("ciclo $i: ${e.javaClass.simpleName}")
            } finally {
                try {
                    socket.close()
                } catch (_: IOException) {
                    // já fechado
                }
                cycle.closed.countDown()
            }
        }
    }

    /** Lê até BYE/EOF: HELLO → WELCOME, PING → PONG, drena o resto. */
    private fun handleConnection(socket: Socket, buf: ByteArray, cycleNo: Int, cycle: Cycle) {
        val input = socket.getInputStream()
        val output = socket.getOutputStream()
        val reader = FrameReader()
        var helloSeen = false
        while (true) {
            val n = input.read(buf)
            if (n < 0) return // EOF: fim do ciclo
            when (val r = reader.feed(buf.copyOfRange(0, n))) {
                is ReaderResult.Error -> {
                    markError("ciclo $cycleNo: quadro inválido (${r.error.kind})")
                    return
                }
                is ReaderResult.Ok -> {
                    for (f in r.frames) {
                        when (f.type) {
                            MessageType.HELLO -> {
                                if (helloSeen) {
                                    markError("ciclo $cycleNo: HELLO repetido")
                                    return
                                }
                                helloSeen = true
                                if (!welcomeForHello(output, f.payload, cycleNo)) return
                            }
                            MessageType.PING -> {
                                output.write(FrameWriter.write(
                                    Frame(MessageType.PONG.code, 0, 0, 0, f.payload),
                                ))
                                output.flush()
                            }
                            MessageType.BYE -> return
                            else -> {
                                // Drena VIDEO/STATE/STREAM_CONFIG/SET/STATS/UNKNOWN.
                                // STATE/STREAM_CONFIG só saem do app logo após o
                                // WELCOME aceito → sinal de ciclo estabelecido.
                                if (f.type == MessageType.STATE || f.type == MessageType.STREAM_CONFIG) {
                                    cycle.established.countDown()
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /** Valida o HELLO e responde WELCOME (sem `deviceKey`: nada persiste). */
    private fun welcomeForHello(output: OutputStream, payload: ByteArray, cycleNo: Int): Boolean {
        val hello = try {
            decodeHello(payload)
        } catch (e: Exception) {
            markError("ciclo $cycleNo: HELLO ilegível (${e.javaClass.simpleName})")
            return false
        }
        val pair = hello.auth as? Auth.Pair
        if (pair == null || pair.token != TOKEN) {
            // Auth divergente (ex.: deviceKey de execução anterior) quebra a
            // premissa do teste. Mensagem sem valores secretos.
            markError("ciclo $cycleNo: auth do HELLO difere do par da URI")
            return false
        }
        val welcome = encodeWelcome(
            Welcome(
                accepted = true,
                protocol = ProtocolVersion(1, 0),
                sessionId = UUID.randomUUID().toString(),
                pc = PcInfo(PC_ID, PC_NAME, "test"),
            ),
        )
        return try {
            output.write(FrameWriter.write(Frame(MessageType.WELCOME.code, 0, 0, 0, welcome)))
            output.flush()
            true
        } catch (e: IOException) {
            markError("ciclo $cycleNo: falha de escrita do WELCOME")
            false
        }
    }

    /** Primeiro erro sem segredos vence; a test thread lê após cada ciclo. */
    private fun markError(msg: String) {
        if (serverError == null) serverError = msg
    }

    private fun wifiIpv4Address(context: Context): Inet4Address {
        val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val wifi = cm.allNetworks.firstOrNull {
            cm.getNetworkCapabilities(it)?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true
        } ?: error("sem rede Wi-Fi para o PC fake")
        return cm.getLinkProperties(wifi)?.linkAddresses
            ?.asSequence()
            ?.mapNotNull { it.address as? Inet4Address }
            ?.firstOrNull { !it.isLoopbackAddress && !it.isLinkLocalAddress }
            ?: error("rede Wi-Fi sem endereço IPv4 para o PC fake")
    }

    private companion object {
        const val TAG = "ConnectStopCyclesTest"

        const val CYCLES = 50
        const val TOTAL_TIMEOUT_MS = 900_000L // `@Test.timeout` é `long` no JUnit 4.13.2
        const val WELCOME_WAIT_MS = 15_000L
        const val CLOSE_WAIT_MS = 10_000L
        const val HOLD_MS = 300L
        const val SETTLE_MS = 200L
        const val PSS_SAMPLE_EVERY = 10
        const val PSS_MAX_GROWTH_KB = 40_000L
        const val JOIN_TIMEOUT_MS = 15_000L
        const val READ_BUF_BYTES = 64 * 1024
        const val FAKE_PC_THREAD = "lc-fake-pc"

        /** Token fictício de 32 hex (não é logado; só compõe a URI). */
        const val TOKEN = "0123456789abcdef0123456789abcdef"

        /** pcId fixo (UUID) da execução; esquecido no finally. */
        const val PC_ID = "a3f7c9d1-5b2e-4c8f-9a6d-1e0f2b3c4d5e"
        const val PC_NAME = "FakePC"
    }
}
