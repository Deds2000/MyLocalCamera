package com.localcamera.app.store

import android.content.Context
import android.util.Log
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.UUID

/**
 * PC conhecido — **não é segredo** (F2-T05): vive em JSON plano em
 * `filesDir/pairings.json` (escrita atômica). A `deviceKey` 256 bits **não**
 * faz parte deste objeto: ela só trafega pelo [SecureStore]
 * (KeystoreSecureStore), com keyId `deviceKey:<pcId>`.
 *
 * [toString] do data class só expõe campos públicos seguros — nunca há chave.
 */
data class KnownPc(
    val pcId: String,
    val name: String,
    val hosts: List<String>,
    val port: Int,
    /** Último acesso (epoch millis); `null` = nunca conectado. */
    val lastAccessed: Long?,
)

/**
 * Dados persistentes do celular (F2-T05):
 * - [deviceId]: UUID estável, gerado uma vez e persistido imediatamente;
 * - PCs conhecidos: JSON plano (`pairings.json`), sem segredos;
 * - `deviceKey` por pcId: via [secureStore] (NUNCA em texto claro, logs ou
 *   `toString` — ver UT-2.07).
 *
 * Robustez: arquivo ausente → estado vazio; arquivo corrompido → loga
 * (mensagem curta, sem dados) e estado vazio, sem crash; o próximo
 * `upsertPc`/`deviceId` regrava o arquivo válido.
 *
 * O construtor primário recebe o diretório para os testes JVM (UT-2.07); o
 * construtor secundário é o uso em produção: `PairingStore(context)`.
 */
class PairingStore(
    private val dir: File,
    private val secureStore: SecureStore,
    private val log: (String) -> Unit = { msg -> Log.w(TAG, msg) },
) {

    constructor(
        context: Context,
        secureStore: SecureStore = KeystoreSecureStore(context),
    ) : this(context.filesDir, secureStore)

    private val file = File(dir, FILE_NAME)
    private var deviceId: String? = null
    private val pcs = LinkedHashMap<String, KnownPc>()

    init {
        load()
    }

    /** ID estável deste celular (UUID v4 minúsculo). Gerado e persistido 1x. */
    @Synchronized
    fun deviceId(): String {
        deviceId?.let { return it }
        val fresh = UUID.randomUUID().toString().lowercase()
        deviceId = fresh
        save() // persiste na hora: ID estável mesmo se o app cair em seguida
        return fresh
    }

    /** Adiciona ou substitui o PC por `pcId`. */
    @Synchronized
    fun upsertPc(pc: KnownPc) {
        val existing = pcs[pc.pcId]
        pcs[pc.pcId] = if (pc.lastAccessed == null && existing != null) {
            pc.copy(lastAccessed = existing.lastAccessed) // não reverter o último acesso
        } else {
            pc
        }
        save()
    }

    @Synchronized
    fun listPcs(): List<KnownPc> = pcs.values.toList()

    fun getPc(pcId: String): KnownPc? = pcs[pcId]

    /** Esquece o PC **e** apaga a `deviceKey` dele (idempotente). */
    @Synchronized
    fun forgetPc(pcId: String) {
        if (pcs.remove(pcId) != null) save()
        secureStore.delete(deviceKeyId(pcId))
    }

    /** Apaga só a `deviceKey` (ex.: ao receber `revoked`, F2-T09); o PC fica na lista. */
    fun deleteDeviceKey(pcId: String) {
        secureStore.delete(deviceKeyId(pcId))
    }

    /** Grava a `deviceKey` (256 bits = 32 bytes) via [SecureStore]. */
    @Synchronized
    fun saveDeviceKey(pcId: String, key: ByteArray) {
        require(key.size == DEVICE_KEY_BYTES) {
            "deviceKey deve ter 32 bytes (obteve ${key.size})"
        }
        secureStore.put(deviceKeyId(pcId), key)
    }

    /** Retorna a `deviceKey` do PC ou `null` (ainda não pareado com "Permitir sempre"). */
    fun deviceKey(pcId: String): ByteArray? = secureStore.get(deviceKeyId(pcId))

    // ---------------------------- persistência ----------------------------

    private fun load() {
        if (!file.exists()) return
        try {
            val root = JSONObject(file.readText(Charsets.UTF_8))
            if (root.optInt("v", FILE_V) != FILE_V) {
                log("pairings.json com versão não reconhecida; estado zerado")
                return
            }
            root.optString("deviceId").takeIf { it.isNotBlank() }?.let { deviceId = it }
            val arr = root.optJSONArray("pcs") ?: return
            for (i in 0 until arr.length()) {
                val o = arr.optJSONObject(i) ?: continue
                val pc = KnownPc(
                    pcId = o.optString("pcId"),
                    name = o.optString("name"),
                    hosts = parseHosts(o.optJSONArray("hosts")),
                    port = o.optInt("port"),
                    lastAccessed = if (o.isNull("lastAccessed")) null else o.optLong("lastAccessed"),
                )
                if (pc.pcId.isNotBlank()) pcs[pc.pcId] = pc
            }
        } catch (e: Exception) {
            log("pairings.json corrompido (${e::class.simpleName}); estado zerado")
        }
    }

    private fun save() {
        val root = JSONObject().apply {
            put("v", FILE_V)
            put("deviceId", deviceId())
            val arr = JSONArray()
            for (pc in pcs.values) {
                arr.put(
                    JSONObject().apply {
                        put("pcId", pc.pcId)
                        put("name", pc.name)
                        val hosts = JSONArray()
                        pc.hosts.forEach { hosts.put(it) }
                        put("hosts", hosts)
                        put("port", pc.port)
                        if (pc.lastAccessed != null) put("lastAccessed", pc.lastAccessed)
                    },
                )
            }
            put("pcs", arr)
        }
        atomicWrite(file, root.toString())
    }

    private fun parseHosts(arr: JSONArray?): List<String> {
        if (arr == null) return emptyList()
        val out = ArrayList<String>(arr.length())
        for (i in 0 until arr.length()) out.add(arr.optString(i))
        return out
    }

    private fun atomicWrite(target: File, content: String) {
        val tmp = File(target.parentFile, target.name + ".tmp")
        tmp.writeText(content, Charsets.UTF_8)
        if (!tmp.renameTo(target)) {
            // Windows: renameTo falha se o destino já existe; Android sobrescreve.
            target.delete()
            if (!tmp.renameTo(target)) {
                tmp.delete()
                error("não foi possível gravar ${target.name}")
            }
        }
    }

    private fun deviceKeyId(pcId: String): String = "$KEY_ID_PREFIX$pcId"

    companion object {
        private const val TAG = "PairingStore"
        private const val FILE_NAME = "pairings.json"
        private const val FILE_V = 1
        private const val DEVICE_KEY_BYTES = 32
        private const val KEY_ID_PREFIX = "deviceKey:"
    }
}
