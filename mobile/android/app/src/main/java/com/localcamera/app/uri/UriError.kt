package com.localcamera.app.uri

/**
 * Erros tipados do parser da URI de pareamento (`localcamera://connect`, F2-T05a).
 *
 * Comportamento de enum: apenas dois resultados possíveis. [kind] corresponde
 * às strings de `expect` dos vetores canônicos (`docs/pairing-uri-vectors.json`):
 * `invalid` e `incompatible_version` — a UI mapeia [kind] para texto i18n.
 *
 * Sealed class (e não `enum`) porque a falha do `Result` (stdlib Kotlin 2.4)
 * encapsula uma instância de `Throwable` — o parser devolve
 * `Result<PairingUri>` com instância de [UriError] — mesmo padrão de
 * `protocol.CodecError`.
 */
sealed class UriError(val kind: String, message: String) : Exception(message) {
    /** Scheme/host errado ou parâmetro obrigatório ausente/malformado. */
    data object Invalid : UriError("invalid", "URI de pareamento inválida")

    /** `v` maior que a suportada (1): nunca conectar; o app precisa ser atualizado. */
    data object VersionTooNew : UriError("incompatible_version", "versão da URI não suportada: atualize o app")
}
