package com.localcamera.app.net

import java.util.concurrent.atomic.AtomicReference

/** Cache da rede ativa: um novo `onAvailable` substitui uma referência obsoleta. */
internal class NetworkCache<T : Any> {
    private val current = AtomicReference<T?>(null)

    fun get(): T? = current.get()

    /** Retorna `true` quando a referência ativa mudou. */
    fun onAvailable(network: T): Boolean = current.getAndSet(network) !== network

    /** CAS por identidade: um `onLost` atrasado não apaga uma rede mais nova. */
    fun onLost(network: T): Boolean = current.compareAndSet(network, null)

    fun clear() {
        current.set(null)
    }
}
