//! Eventos emitidos pelo servidor para a camada acima (Tauri/UI).

/// Motivo do fechamento de uma conexão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseReason {
    /// O par fechou a conexão (EOF limpo).
    PeerClosed,
    /// O primeiro quadro (`HELLO`) não chegou dentro do prazo.
    HelloTimeout,
    /// Erro do protocolo (magic inválido, payload acima do limite).
    ProtocolError,
    /// Estourou o limite de conexões não autenticadas.
    TooManyUnauth,
    /// Shutdown do servidor.
    Shutdown,
    /// Substituída por outra conexão do mesmo dispositivo.
    Replaced,
    /// Erro de transporte (reset, erro de socket).
    ConnectionLost,
    /// Handshake recusado (`bad_token`, `not_approved`, `revoked`,
    /// `busy`, `version_mismatch`, `bad_audio_ticket`); o `WELCOME`
    /// com o motivo é enviado antes do fechamento.
    AuthDenied,
}

/// Eventos do servidor, na ordem de emissão.
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// Listener ativo na porta efetiva.
    Listening { port: u16 },
    /// A porta pedida estava ocupada; o servidor usa outra.
    PortChanged { requested: u16, actual: u16 },
    /// Nova conexão aceita (aguardando o primeiro quadro).
    ConnectionOpened {
        conn_id: u64,
        peer: std::net::SocketAddr,
    },
    /// Conexão encerrada.
    ConnectionClosed { conn_id: u64, reason: CloseReason },
    /// Sessão iniciada (handshake aceito e `WELCOME` entregue).
    SessionStarted {
        conn_id: u64,
        session_id: String,
        device_id: String,
        device_name: String,
        peer: std::net::SocketAddr,
        caps: lc_protocol::json::Caps,
    },
    /// Sessão encerrada (a conexão estava `Active`).
    SessionEnded {
        session_id: String,
        reason: CloseReason,
    },
}
