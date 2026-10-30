package com.localcamera.app.store

import android.content.Context
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Log
import java.io.File
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * [SecureStore] em produção: AES-GCM com a chave AES-256 no `AndroidKeyStore`.
 *
 * **Decisão (F2-T05, passo 1 — verificada na doc oficial do Android,
 * developer.android.com, vigente para targetSdk 36):**
 * - Sem lib de terceiros ("armazenamento seguro" de terceiros foi descontinuada
 *   em vários casos; cartão F2-T05 proíbe adicioná-la sem ADR). Usamos o
 *   Keystore diretamente, com criptografia mínima própria:
 *   [KeyGenerator] (provider `AndroidKeyStore`) + [KeyGenParameterSpec] com
 *   `AES`, 256 bits, `BLOCK_MODE_GCM` + `ENCRYPTION_PADDING_NONE`
 *   (transformação `AES/GCM/NoPadding`), alias fixo [KEY_ALIAS].
 * - A chave é **não exportável**: o Keystore guarda o material em HW/TEE e
 *   nunca devolve os bytes — só opera com ele. O que vai para o disco é o
 *   ciphertext (IV aleatória de 12 bytes + GCM tag de 128 bits embutidos).
 * - A chave NÃO exige autenticação do usuário (o app roda com o app em
 *   segundo plano, sem biometria). `setUserAuthenticationRequired(false)` foi
 *   **deprecado na API 30**: em API ≥ 30 usamos
 *   `setUserAuthenticationParameters(0, 0)` (0 = sem autenticação, sem
 *   validade); em API < 30 (minSdk 24) usamos o setter legado.
 *   (`setUnlockedDeviceRequired` também é só ≥ 30 e não é o que queremos:
 *   travaria por lockscreen bloquearia uso com o aparelho bloqueado.)
 * - Um único alias para todos os segredos: o isolamento entre segredos vem da
 *   GCM (tag por blob). Os ciphertexts ficam em `filesDir/secrets/<keyId>.enc`
 *   (escrita atômica: temp + rename) — arquivos privados do app, já fora do
 *   backup por `allowBackup="false"` (ST-2.01).
 *
 * Segredos nunca aparecem em log (só a mensagem curta de falha) nem em
 * `toString` (a classe não tem `toString` com dados).
 */
class KeystoreSecureStore(context: Context) : SecureStore {

    private val dir: File = File(context.filesDir, "secrets")
    private val random = SecureRandom()
    private val keyLock = Any()
    private var cachedKey: SecretKey? = null

    override fun put(keyId: String, secret: ByteArray) {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, loadOrCreateKey())
        val iv = cipher.iv // 12 bytes aleatórios gerados pelo Keystore
        val ciphertext = cipher.doFinal(secret)
        val payload = ByteArray(iv.size + ciphertext.size).also {
            iv.copyInto(it, 0)
            ciphertext.copyInto(it, iv.size)
        }
        atomicWrite(File(dir, fileName(keyId)), payload)
    }

    override fun get(keyId: String): ByteArray? {
        val file = File(dir, fileName(keyId))
        if (!file.exists()) return null
        return try {
            val payload = file.readBytes()
            if (payload.size <= GCM_IV_BYTES) {
                Log.w(TAG, "blob de $keyId malformado (curto demais)")
                null
            } else {
                val cipher = Cipher.getInstance(TRANSFORMATION)
                cipher.init(
                    Cipher.DECRYPT_MODE,
                    loadOrCreateKey(),
                    GCMParameterSpec(GCM_TAG_BITS, payload, 0, GCM_IV_BYTES),
                )
                cipher.doFinal(payload, GCM_IV_BYTES, payload.size - GCM_IV_BYTES)
            }
        } catch (e: Exception) {
            // Corrompido/inválido → trata como ausente (sem crash, sem segredo no log).
            Log.w(TAG, "falha ao decifrar $keyId: ${e::class.simpleName}")
            null
        }
    }

    override fun delete(keyId: String) {
        File(dir, fileName(keyId)).delete()
    }

    private fun loadOrCreateKey(): SecretKey {
        cachedKey?.let { return it }
        synchronized(keyLock) {
            cachedKey?.let { return it }
            val store = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
            (store.getKey(KEY_ALIAS, null) as? SecretKey)?.let {
                cachedKey = it
                return it
            }
            val spec = KeyGenParameterSpec.Builder(
                KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .apply {
                    if (Build.VERSION.SDK_INT >= 30) {
                        // API 30+: substituto do setUserAuthenticationRequired(false)
                        // (deprecado). 0 = sem autenticação exigida.
                        setUserAuthenticationParameters(0, 0)
                    } else {
                        @Suppress("DEPRECATION") // API 23–29: só existe o setter legado.
                        setUserAuthenticationRequired(false)
                    }
                }
                .build()
            val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
            generator.init(spec)
            val key = generator.generateKey()
            cachedKey = key
            return key
        }
    }

    /** keyId → nome de arquivo seguro (o keyId é só um rótulo, não é segredo). */
    private fun fileName(keyId: String): String =
        keyId.replace(Regex("[^A-Za-z0-9._-]"), "_") + ".enc"

    private fun atomicWrite(file: File, payload: ByteArray) {
        if (!dir.exists() && !dir.mkdirs()) {
            error("não foi possível criar ${dir.path}")
        }
        val tmp = File(dir, file.name + ".tmp")
        tmp.writeBytes(payload)
        if (!tmp.renameTo(file)) {
            // Windows: renameTo falha se o destino já existe; Android/Linux sobrescreve.
            file.delete()
            if (!tmp.renameTo(file)) {
                tmp.delete()
                error("não foi possível gravar ${file.name}")
            }
        }
    }

    companion object {
        private const val TAG = "KeystoreSecureStore"
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val KEY_ALIAS = "localcamera.devicekeys"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val GCM_IV_BYTES = 12
        private const val GCM_TAG_BITS = 128
    }
}
