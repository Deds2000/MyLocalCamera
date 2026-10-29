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
}
