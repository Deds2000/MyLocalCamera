package com.localcamera.app.service

import com.localcamera.app.store.KnownPc
import com.localcamera.app.store.PairingStore
import com.localcamera.app.store.SecureStore
import com.localcamera.app.uri.PairingUri
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.util.concurrent.ConcurrentHashMap

class KnownPcPersistenceTest {

    @get:Rule
    val temporaryFolder = TemporaryFolder()

    @Test
    fun onlyPersistentlyPairedPcIsSavedForReconnect() {
        val secureStore = MemorySecureStore()
        val store = PairingStore(temporaryFolder.root, secureStore) {}
        val target = PairingUri.parse(
            "localcamera://connect?v=1&hosts=192.168.0.10&port=48650&token=${"0".repeat(32)}",
        ).getOrThrow()
        val logs = mutableListOf<String>()
        val key = ByteArray(32) { 0x5A }

        KnownPcPersistence.remember(store, target, PC_ID, "Desktop", logs::add)
        assertTrue("aprovação única não deve virar PC reconectável", store.listPcs().isEmpty())

        store.saveDeviceKey(PC_ID, key)
        KnownPcPersistence.remember(store, target, PC_ID, "Desktop", logs::add)

        assertEquals(
            KnownPc(PC_ID, "Desktop", listOf("192.168.0.10"), 48650, store.listPcs().single().lastAccessed),
            store.listPcs().single(),
        )
        assertNotNull(store.deviceKey(PC_ID))
        assertTrue("não deve logar ao salvar", logs.isEmpty())
    }

    private class MemorySecureStore : SecureStore {
        private val values = ConcurrentHashMap<String, ByteArray>()
        override fun put(keyId: String, secret: ByteArray) { values[keyId] = secret.copyOf() }
        override fun get(keyId: String): ByteArray? = values[keyId]?.copyOf()
        override fun delete(keyId: String) { values.remove(keyId) }
    }

    private companion object {
        const val PC_ID = "11111111-1111-4111-8111-111111111111"
    }
}
