package com.localcamera.app.store

/**
 * Armazenamento de segredos (F2-T05).
 *
 * Implementação de produção: [KeystoreSecureStore] (AES-GCM com chave AES-256
 * no `AndroidKeyStore`, nunca exportável). A interface existe para os testes
 * de unidade JVM usarem um fake (UT-2.07) — o Keystore em si só existe no
 * Android (sem provider no JVM).
 *
 * Contrato:
 * - segredos NUNCA são aceitos/retornados em log nem em `toString`;
 * - [get] retorna `null` se o segredo não existe ou não pôde ser decifrado;
 * - [delete] é idempotente (segredo ausente → nada acontece).
 */
interface SecureStore {
    /** Grava (ou sobrescreve) o segredo identificado por [keyId]. */
    fun put(keyId: String, secret: ByteArray)

    /** Retorna o segredo identificado por [keyId] ou `null`. */
    fun get(keyId: String): ByteArray?

    /** Remove o segredo identificado por [keyId] (idempotente). */
    fun delete(keyId: String)
}
