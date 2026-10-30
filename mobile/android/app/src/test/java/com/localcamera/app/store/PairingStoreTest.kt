package com.localcamera.app.store

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.util.concurrent.ConcurrentHashMap

/**
 * UT-2.07 (F2-T05) — `PairingStore` no JVM:
 * - grava/lê PC e `deviceKey` (via [FakeSecureStore], que simula o blob
 *   criptografado persistido pelo `KeystoreSecureStore`);
 * - `forgetPc` apaga o PC **e** a chave; `deleteDeviceKey` apaga só a chave;
 * - arquivo corrompido → loga e estado vazio, sem crash;
 * - a chave NUNCA aparece em texto claro no arquivo, nos logs nem em
 *   `toString` dos dados.
 *
 * O `KeystoreSecureStore` de verdade não roda no JVM (o provider
 * `AndroidKeyStore` só existe no Android) — por isso o fake; a cobertura
 * em aparelho vem do ST-2.01 (adb backup).
 */
class PairingStoreTest {

    @get:Rule
    val tmp = TemporaryFolder()

    private lateinit var fake: FakeSecureStore
    private val logs = mutableListOf<String>()

    @Before
    fun setup() {
        fake = FakeSecureStore()
        logs.clear()
    }

    private fun newStore(): PairingStore = PairingStore(tmp.root, fake) { logs.add(it) }

    /** Simula o `KeystoreSecureStore`: guarda os bytes por keyId, em "memória". */
    private class FakeSecureStore : SecureStore {
        private val blobs = ConcurrentHashMap<String, ByteArray>()
        override fun put(keyId: String, secret: ByteArray) { blobs[keyId] = secret.copyOf() }
        override fun get(keyId: String): ByteArray? = blobs[keyId]?.copyOf()
        override fun delete(keyId: String) { blobs.remove(keyId) }
    }

    companion object {
        private val PC1 = "11111111-1111-4111-8111-111111111111"
        private val PC2 = "22222222-2222-4222-8222-222222222222"
        private val UUID_RE =
            Regex("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")

        // 32 bytes determinísticos; os 16 primeiros formam uma assinatura
        // legível para buscar texto claro ("ABCDEFGHIJKLMNOP").
        private val KEY: ByteArray = ByteArray(32) { (65 + it).toByte() }
        private val KEY_PLAIN_MARKER = String(KEY.copyOfRange(0, 16), Charsets.ISO_8859_1)

        private fun pc(pcId: String = PC1, port: Int = 48650) = KnownPc(
            pcId = pcId,
            name = "DESKTOP-TESTE",
            hosts = listOf("192.168.0.10"),
            port = port,
            lastAccessed = null,
        )
    }

    @Test
    fun deviceIdIsStableAcrossInstances() {
        val id1 = newStore().deviceId()
        val id2 = newStore().deviceId()
        assertEquals("deviceId deve ser estável entre instâncias", id1, id2)
        assertTrue("deviceId deve ser UUID minúsculo: $id1", UUID_RE.matches(id1))
    }

    @Test
    fun upsertPcPersistsAndReplaces() {
        val s = newStore()
        s.upsertPc(pc().copy(lastAccessed = 1000L))

        val reloaded = newStore()
        assertEquals(1, reloaded.listPcs().size)
        assertEquals("DESKTOP-TESTE", reloaded.getPc(PC1)?.name)
        assertEquals(1000L, reloaded.getPc(PC1)?.lastAccessed)

        // upsert substitui; lastAccessed ausente não zera o último acesso
        reloaded.upsertPc(pc(port = 50000))
        val updated = reloaded.getPc(PC1)!!
        assertEquals(50000, updated.port)
        assertEquals(1000L, updated.lastAccessed)

        // dois PCs coexistem
        reloaded.upsertPc(pc(pcId = PC2))
        assertEquals(2, reloaded.listPcs().size)
    }

    @Test
    fun deviceKeyRoundtripThroughSecureStore() {
        val s = newStore()
        s.saveDeviceKey(PC1, KEY)
        assertArrayEquals(KEY, s.deviceKey(PC1))

        // nova instância do app lê a chave do mesmo "blob" persistido
        val reloaded = newStore()
        assertArrayEquals(KEY, reloaded.deviceKey(PC1))
        assertNull("PC sem chave não retorna chave", reloaded.deviceKey(PC2))
    }

    @Test
    fun forgetPcRemovesPcAndKey() {
        val s = newStore()
        s.upsertPc(pc())
        s.saveDeviceKey(PC1, KEY)
        s.forgetPc(PC1)

        assertNull(s.getPc(PC1))
        assertTrue("lista deve ficar vazia", s.listPcs().isEmpty())
        assertNull("chave deve ter sido apagada", s.deviceKey(PC1))
    }

    @Test
    fun deleteDeviceKeyKeepsPc() {
        val s = newStore()
        s.upsertPc(pc())
        s.saveDeviceKey(PC1, KEY)
        s.deleteDeviceKey(PC1)

        assertNotNull("o PC fica na lista", s.getPc(PC1))
        assertNull("só a chave é apagada", s.deviceKey(PC1))
    }

    @Test
    fun corruptedFileYieldsEmptyStateWithoutCrash() {
        val s0 = newStore()
        s0.upsertPc(pc())
        s0.saveDeviceKey(PC1, KEY)
        val pairings = File(tmp.root, "pairings.json")
        assertTrue(pairings.exists())

        pairings.writeText("{{ isto não é JSON válido")
        val s = newStore()

        assertEquals("estado deve ficar vazio", emptyList<KnownPc>(), s.listPcs())
        assertTrue("deve logar o corrompimento", logs.any { it.contains("corrompido") })
        val freshId = s.deviceId()
        assertTrue("deviceId novo deve ser UUID", UUID_RE.matches(freshId))

        // e a próxima escrita regrava um arquivo válido
        s.upsertPc(pc())
        val reloaded = newStore()
        assertNotNull(reloaded.getPc(PC1))
        assertEquals(freshId, reloaded.deviceId())
    }

    @Test
    fun keyNeverAppearsInFileLogsOrToString() {
        val s = newStore()
        s.upsertPc(pc().copy(lastAccessed = 1234L))
        s.saveDeviceKey(PC1, KEY)
        // força logs (corrompe e relê para exercitar o caminho de log)
        File(tmp.root, "pairings.json").writeText("{corrompido")
        newStore()

        val diskText = File(tmp.root, "pairings.json").readText(Charsets.UTF_8)
        val logText = logs.joinToString("\n")
        val dataText = s.listPcs().toString()

        assertFalse("chave em texto claro no arquivo", diskText.contains(KEY_PLAIN_MARKER))
        assertFalse("chave em texto claro nos logs", logText.contains(KEY_PLAIN_MARKER))
        assertFalse("chave em texto claro no toString dos dados", dataText.contains(KEY_PLAIN_MARKER))
    }

    @Test
    fun saveDeviceKeyRejectsWrongSize() {
        val s = newStore()
        assertThrows(IllegalArgumentException::class.java) {
            s.saveDeviceKey(PC1, ByteArray(16))
        }
        assertThrows(IllegalArgumentException::class.java) {
            s.saveDeviceKey(PC1, ByteArray(33))
        }
    }
}
