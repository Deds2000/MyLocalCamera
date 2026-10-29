//! Negociação de versão do protocolo no handshake.

use lc_protocol::json::ProtocolVersion;

/// Versão de protocolo do servidor (ver `docs/CHEATSHEET.md`).
pub const SERVER_PROTOCOL: ProtocolVersion = ProtocolVersion { major: 1, minor: 0 };

/// Negocia a versão informada no `HELLO`.
///
/// `Ok` se o `major` do cliente é igual ao do servidor; `minor` diferente
/// é aceito (novidades são opt-in do cliente). `Err` carrega a menor
/// versão suportada pelo servidor (campo `minSupported` do `WELCOME`).
pub fn negotiate(client: &ProtocolVersion) -> Result<(), ProtocolVersion> {
    if client.major == SERVER_PROTOCOL.major {
        Ok(())
    } else {
        Err(SERVER_PROTOCOL)
    }
}
