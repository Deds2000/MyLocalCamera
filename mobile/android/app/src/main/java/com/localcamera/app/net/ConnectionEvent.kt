package com.localcamera.app.net

import com.localcamera.app.protocol.PcInfo

/**
 * Eventos de [PrimaryConnection] para o chamador (F2-T12).
 *
 * Só códigos e dados estruturados — nunca frases vindas da rede: a UI
 * traduz por código (`§6.6` do protocolo). Nenhum segredo trafega por
 * aqui: `deviceKey`/`audioTicket` vão para o `PairingStore`/Fase 8.
 */
sealed class ConnectionEvent {

    /**
     * PC aguardando aprovação no desktop (`PENDING{timeoutS}` — o celular
     * mostra "Aguardando aprovação no PC…"). Sem `PING`/liveness nesse
     * intervalo: o PC só responde `PONG` com a sessão ativa.
     */
    data class Pending(val timeoutS: Int) : ConnectionEvent()

    /** `WELCOME` aceito: o chamador envia `STATE` + `STREAM_CONFIG` (F2-T12). */
    data class Welcome(val sessionId: String, val pc: PcInfo) : ConnectionEvent()

    /**
     * `WELCOME` recusado. [reason] é o código do protocolo:
     * `bad_token`, `not_approved`, `revoked`, `busy`, `version_mismatch`,
     * `bad_audio_ticket`, `timeout`, `protocol_error` (desconhecido → mantido).
     * `revoked`/`version_mismatch` são definitivos (sem reconectar).
     */
    data class Denied(val reason: String) : ConnectionEvent()

    /**
     * Conexão perdida (sem `PONG` por 6 s, peer fechou ou falha de
     * leitura/escrita na sessão): a conexão reconecta com backoff.
     */
    data object Lost : ConnectionEvent()

    /** `REQUEST_KEYFRAME` do PC: o encoder deve responder em ≤300 ms (FT-2.13). */
    data object KeyframeRequested : ConnectionEvent()

    /** `BYE{reason}` do PC: encerramento definitivo (sem reconectar). */
    data class Bye(val reason: String) : ConnectionEvent()

    /**
     * `SET` (PC→celular): payload JSON para o `StateReducer` aplicar
     * (F2-T12) e responder com `STATE` (protocolo §9).
     */
    data class Set(val json: String) : ConnectionEvent()
}
