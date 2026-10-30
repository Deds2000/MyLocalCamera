package com.localcamera.app.uri

import java.net.URLDecoder

/**
 * URI de pareamento decodificada:
 * `localcamera://connect?v=1&hosts=IP1,IP2&port=48650&token=<32 hex>&pc=<pcId>&name=<nome>`.
 *
 * Mesmas regras do parser de referência TS
 * (`packages/shared/src/protocol/pairing-uri.ts`), validadas pelos vetores
 * `docs/pairing-uri-vectors.json` (F2-T05a):
 * - scheme `localcamera`, host `connect`;
 * - `v` inteiro obrigatório; `v > 1` → [UriError.VersionTooNew] (nunca conectar);
 * - `hosts` 1–4 IPv4 (quadros 0–255, sem zero à esquerda);
 * - `port` 1–65535 (padrão 48650 quando ausente);
 * - `token` exatamente 32 hex minúsculos;
 * - `pc` UUID quando presente (null quando ausente);
 * - `name` não vazia, ≤ 64 caracteres após decodificar (null quando ausente);
 * - parâmetros desconhecidos são ignorados (primeira ocorrência vence).
 *
 * [parse] nunca lança exceção: qualquer entrada malformada vira `Result.failure`
 * (ST-2.04 em F2-T14 alimenta o parser com URIs maliciosas).
 *
 * Obs.: no stdlib do Kotlin 2.4 o `Result` tem um único parâmetro de tipo
 * (`Result<T>`; o parâmetro de erro `E` foi removido) — por isso a API é
 * `Result<PairingUri>` e a falha carrega a instância de [UriError]
 * (`r.exceptionOrNull()`).
 */
data class PairingUri(
    val v: Int,
    val hosts: List<String>,
    val port: Int,
    val token: String,
    val pc: String?,
    val name: String?,
) {
    companion object {
        /** Versão máxima suportada por este parser. */
        const val SUPPORTED_V = 1
        /** Porta padrão do desktop quando a URI não traz `port`. */
        const val DEFAULT_PORT = 48650
        const val MAX_HOSTS = 4
        const val MAX_NAME_LEN = 64

        private const val SCHEME = "localcamera://"
        private const val HOST = "connect"

        // Mesma regex IPv4 da referência TS (rejeita zero à esquerda, ex.: `01.2.3.4`).
        private val IPV4 = Regex(
            "^(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])" +
                "(\\.(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])){3}$",
        )
        private val UUID_RE = Regex(
            "^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
        )
        private val DIGITS = Regex("^[0-9]+$")

        /**
         * Faz o parse de uma URI de pareamento. Nunca lança: entrada malformada →
         * `failure([UriError.Invalid])`; `v` acima de [SUPPORTED_V] →
         * `failure([UriError.VersionTooNew])`.
         */
        @JvmStatic
        fun parse(raw: String): Result<PairingUri> = try {
            parseInternal(raw)
        } catch (t: Throwable) {
            Result.failure(UriError.Invalid)
        }

        private fun parseInternal(raw: String): Result<PairingUri> {
            if (!raw.startsWith(SCHEME)) return fail()
            val body = raw.substring(SCHEME.length).substringBefore('#')
            val qIndex = body.indexOf('?')
            val host = if (qIndex < 0) body else body.substring(0, qIndex)
            if (host != HOST) return fail()
            val query = if (qIndex < 0) "" else body.substring(qIndex + 1)

            // Primeiro parâmetro com a mesma chave vence (comportamento do URLSearchParams).
            val p = HashMap<String, String>()
            for (pair in query.split('&')) {
                if (pair.isEmpty()) continue
                val eq = pair.indexOf('=')
                val key = if (eq < 0) pair else pair.substring(0, eq)
                val value = if (eq < 0) "" else pair.substring(eq + 1)
                if (key !in p) p[key] = urlDecode(value)
            }

            // v (obrigatório): inteiro; > 1 → VersionTooNew.
            val vRaw = p["v"] ?: return fail()
            if (!DIGITS.matches(vRaw)) return fail()
            val vDigits = vRaw.trimStart('0')
            if (vDigits.length > 1) return versionTooNew() // ≥ 10, sempre > suportada
            val v = if (vDigits.isEmpty()) 0 else vDigits[0].digitToInt()
            if (v > SUPPORTED_V) return versionTooNew()

            // hosts (obrigatório): 1–4 IPv4.
            val hosts = (p["hosts"] ?: return fail()).split(',')
            if (hosts.size > MAX_HOSTS || hosts.any { !IPV4.matches(it) }) return fail()

            // port (opcional; padrão DEFAULT_PORT): 1–65535.
            val port = when (val raw = p["port"]) {
                null -> DEFAULT_PORT
                else -> {
                    if (!DIGITS.matches(raw)) return fail()
                    val digits = raw.trimStart('0')
                    if (digits.length > 5) return fail() // ≥ 100000 > 65535
                    val value = if (digits.isEmpty()) 0 else digits.toInt()
                    if (value !in 1..65535) return fail()
                    value
                }
            }

            // token (obrigatório): exatamente 32 hex minúsculos.
            val token = p["token"] ?: return fail()
            if (token.length != 32 || token.any { it !in '0'..'9' && it !in 'a'..'f' }) return fail()

            // pc (opcional): UUID quando presente.
            val pc: String? = when (val raw = p["pc"]) {
                null -> null
                else -> if (UUID_RE.matches(raw)) raw else return fail()
            }

            // name (opcional): não vazia, ≤ 64 caracteres decodificados.
            val name: String? = when (val raw = p["name"]) {
                null -> null
                else -> if (raw.isEmpty() || raw.length > MAX_NAME_LEN) return fail() else raw
            }

            return Result.success(PairingUri(v, hosts, port, token, pc, name))
        }

        private fun fail(): Result<PairingUri> = Result.failure(UriError.Invalid)

        private fun versionTooNew(): Result<PairingUri> = Result.failure(UriError.VersionTooNew)

        /**
         * Percent-decoding UTF-8, com o mesmo comportamento do URLSearchParams
         * (`+` vira espaço; sequências inválidas, ex. `%C3` truncado, ficam literais).
         */
        private fun urlDecode(value: String): String = URLDecoder.decode(value, "UTF-8")
    }
}
