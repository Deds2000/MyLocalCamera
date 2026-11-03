package com.localcamera.app.service

import com.localcamera.app.store.KnownPc
import com.localcamera.app.store.PairingStore
import com.localcamera.app.uri.PairingUri

/** Persiste metadados somente quando a conexão dispõe de `deviceKey`. */
internal object KnownPcPersistence {

    fun remember(
        store: PairingStore,
        target: PairingUri,
        pcId: String,
        name: String,
        log: (String) -> Unit,
    ) {
        try {
            if (store.deviceKey(pcId) == null) return // aprovação única, sem chave persistente
            store.upsertPc(
                KnownPc(
                    pcId = pcId,
                    name = name,
                    hosts = target.hosts,
                    port = target.port,
                    lastAccessed = System.currentTimeMillis(),
                ),
            )
        } catch (e: Exception) {
            log("PC conhecido não salvo (${e::class.simpleName})") // nunca registra segredo/dados do peer
        }
    }
}
