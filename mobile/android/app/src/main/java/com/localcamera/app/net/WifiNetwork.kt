package com.localcamera.app.net

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.util.Log
import java.net.Socket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

/**
 * Vínculo de rede Wi-Fi do socket principal (F2-T16, targetSdk 36).
 *
 * **Pesquisa (passo 1 do cartão) — conferida na fonte do SDK (estub do
 * `compileSdk 36`) e nos docs oficiais de `ConnectivityManager`/`Network`:**
 *
 * - **Abordagem: `requestNetwork` + `Network` por socket** (via
 *   `Network.getSocketFactory()`). `ConnectivityManager.requestNetwork(NetworkRequest,
 *   NetworkCallback)` é o pedido *ativo*: `onAvailable(network)` ao casar,
 *   `onUnavailable()` se nada casar — nenhum dos dois é deprecated.
 *   O `NetworkRequest` usa **só** `addTransportType(TRANSPORT_WIFI)`, **sem**
 *   `NET_CAPABILITY_*`: assim o Wi-Fi da LAN casa **mesmo sem internet
 *   validada** (o caso FT-2.11, rede isolada). Pedir `NET_CAPABILITY_INTERNET`
 *   ou `_VALIDATED` quebraria o cenário-alvo — por isso não pedimos.
 * - **Por que não `bindProcessToNetwork(Network)`:** o próprio javadoc oficial
 *   dele (API 36, também não deprecated) recomenda explicitamente o socket
 *   individual: "Using individually bound Sockets created by
 *   Network.getSocketFactory().createSocket() ... is preferred". O bind
 *   processual afetaria **todos** os outros sockets do app (socket de áudio,
 *   F2-T14, etc.); e, se o Wi-Fi caísse, o javadoc de `Network.getSocketFactory()`
 *   avisa que a factory e os sockets "cease to work" — por socket, o raio de
 *   ação fica no socket principal e o laço de reconexão (backoff, F2-T09)
 *   recupera sozinho: `onLost` zera o [current] e o próximo socket re-adquire.
 * - **Liberação:** `unregisterNetworkCallback(callback)` — o overload
 *   `releaseNetworkRequest(Network)` **não existe** mais na superfície da API 36
 *   (sobra só o de `PendingIntent`). O callback fica registrado enquanto a
 *   sessão vive: mantém o pedido ativo e recebe `onLost`/`onAvailable`.
 * - **VPN (nota p/ `docs/devices.md`, observada em F2-T14/F2-T15):** o pedido
 *   casa só o transporte físico `TRANSPORT_WIFI`; uma VPN ativa é rede
 *   `TRANSPORT_VPN` e **não** é casada nem sequestrada — nada de forçar Wi-Fi
 *   quando o usuário está com VPN intencional. Sem Wi-Fi (VPN como rede
 *   padrão ou Wi-Fi desligado), [acquire] devolve `null` e o socket usa a
 *   rede padrão. Com VPN *split*, o tráfego de LAN já sai direto pela
 *   interface, fora do túnel — comportamento a registrar em `docs/devices.md`.
 *
 * Os hosts-alvo chegam como IP literal no QR (`PairingUri`) → sem resolução
 * de nome em rede específica (`Network.getAllByName` não é preciso).
 *
 * Thread-safety: [acquire]/[release]/[createSocket] podem vir de várias
 * threads `Dispatchers.IO` (conexão paralela, F2-T09). Callbacks do sistema
 * usam apenas CAS — **nunca** travam sobre [lock].
 */
class WifiNetwork(
    context: Context,
    private val log: (String) -> Unit = { msg -> Log.i(TAG, msg) },
) {

    private val cm: ConnectivityManager =
        context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager

    /** Wi-Fi em uso; `null` = rede padrão. CAS: `onAvailable`/`onLost`/`acquire`. */
    private val current = AtomicReference<Network?>(null)

    /** Serializa "pedido em voo" (1 `requestNetwork` ativo por vez). */
    private val lock = Any()

    /** Callback do pedido ativo (único — o sistema exige 1 por `requestNetwork`). */
    @Volatile private var activeCallback: ConnectivityManager.NetworkCallback? = null

    /** Último "sem Wi-Fi" (cache negativa): evita re-esperar a cada reconexão. */
    @Volatile private var unavailableSinceMs = 0L

    /** Stop definitivo ([release]): um pré-aquecimento tardio não re-registra. */
    @Volatile private var released = false

    /**
     * O Wi-Fi de LAN para o socket principal, ou `null` (→ **rede padrão**,
     * com log — cartão). Cacheia: o [Network] até `onLost`/[release]; o
     * resultado negativo por [NEGATIVE_CACHE_MS]. Bloqueia no máximo
     * [ACQUIRE_TIMEOUT_MS] quando ainda não há pedido (a fábrica do socket
     * roda dentro do orçamento de 2 s por host de F2-T09).
     */
    fun acquire(): Network? {
        if (released) return null
        current.get()?.let { return it }
        if (System.currentTimeMillis() - unavailableSinceMs < NEGATIVE_CACHE_MS) return null
        return synchronized(lock) {
            if (released) return null
            current.get()?.let { return it } // outra thread já adquiriu
            if (System.currentTimeMillis() - unavailableSinceMs < NEGATIVE_CACHE_MS) return null

            val latch = CountDownLatch(1)
            val callback = object : ConnectivityManager.NetworkCallback() {
                override fun onAvailable(network: Network) {
                    if (current.compareAndSet(null, network)) {
                        unavailableSinceMs = 0L
                        log("Wi-Fi vinculado")
                    }
                    latch.countDown()
                }

                override fun onLost(network: Network) {
                    // Wi-Fi caiu/trocou: zera e o próximo socket re-adquire.
                    if (current.compareAndSet(network, null)) {
                        log("Wi-Fi perdido: rede padrão até re-adquirir")
                        latch.countDown()
                    }
                }

                override fun onUnavailable() = latch.countDown()
            }

            activeCallback?.let { if (it !== callback) unregisterQuietly(it) }
            activeCallback = callback
            var requested = true
            try {
                cm.requestNetwork(wifiRequest(), callback)
            } catch (e: Exception) {
                // Sem permissão/serviço etc.: cai para a rede padrão.
                requested = false
                unregisterQuietly(callback)
                activeCallback = null
                log("requestNetwork falhou (${e.javaClass.simpleName}): rede padrão")
            }

            var net: Network? = null
            if (requested) {
                try {
                    latch.await(ACQUIRE_TIMEOUT_MS, TimeUnit.MILLISECONDS)
                } catch (e: InterruptedException) {
                    Thread.currentThread().interrupt()
                }
                net = current.get()
            }
            if (net == null) {
                // "Nenhuma Wi-Fi" → rede padrão (fallback exigido pelo cartão).
                unregisterQuietly(callback)
                activeCallback = null
                unavailableSinceMs = System.currentTimeMillis()
                log("nenhuma Wi-Fi: usando rede padrão")
            }
            net
        }
    }

    /**
     * Socket pronto para o socket principal: vinculado ao Wi-Fi via
     * `Network.getSocketFactory()` (tráfego **só** por essa rede, antes do
     * `connect`) ou socket comum (rede padrão), se não houver Wi-Fi.
     */
    fun createSocket(): Socket = acquire()?.let { it.getSocketFactory().createSocket() } ?: Socket()

    /** Encerra o pedido (idempotente e definitivo): sockets futuros usam a
     * rede padrão e nenhum [acquire] subsequente re-registra o callback. */
    fun release() {
        val cb = synchronized(lock) {
            val old = activeCallback
            activeCallback = null
            current.set(null)
            unavailableSinceMs = 0L
            released = true
            old
        }
        if (cb != null) unregisterQuietly(cb)
    }

    /**
     * Wi-Fi de LAN: **só** o tipo de transporte. Sem `NET_CAPABILITY_INTERNET`/
     * `_VALIDATED` por decisão registrada na documentação da classe — o Wi-Fi
     * sem internet (FT-2.11) precisa casar.
     */
    private fun wifiRequest(): NetworkRequest =
        NetworkRequest.Builder().addTransportType(NetworkCapabilities.TRANSPORT_WIFI).build()

    private fun unregisterQuietly(cb: ConnectivityManager.NetworkCallback) {
        try {
            cm.unregisterNetworkCallback(cb)
        } catch (e: Exception) {
            // já desregistrado (app fechando / `onLost` final)
        }
    }

    private companion object {
        private const val TAG = "WifiNetwork"

        /** Espera por Wi-Fi casando (a fábrica do socket tem 2 s por host, F2-T09). */
        private const val ACQUIRE_TIMEOUT_MS = 1500L

        /** Cache do "sem Wi-Fi" (reconexões em rajada não re-esperam). */
        private const val NEGATIVE_CACHE_MS = 2000L
    }
}
