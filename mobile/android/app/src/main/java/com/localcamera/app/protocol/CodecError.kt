package com.localcamera.app.protocol

/**
 * Erros do codec. [kind] corresponde às strings usadas nos vetores
 * (`docs/protocol-vectors.json`): `BadMagic`, `TooLarge`, `BadJson`.
 */
sealed class CodecError(val kind: String, message: String) : Exception(message) {
    /** Magic inválido (bytes 0–1 diferentes de `43 4C`). */
    data object BadMagic : CodecError("BadMagic", "magic inválido")

    /** `length` do cabeçalho (ou payload na escrita) acima do limite de 8 MiB. */
    class TooLarge(val size: Long) : CodecError("TooLarge", "payload grande demais: $size bytes")

    /** JSON malformado, campo obrigatório ausente ou campo com tipo errado. */
    class BadJson(message: String) : CodecError("BadJson", message)
}
