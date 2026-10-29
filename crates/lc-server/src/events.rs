//! Eventos emitidos pelo servidor para a camada acima (Tauri/UI).

/// Pedido de aprovação de um novo pareamento (dispositivo desconhecido).
///
/// Sem segredos: o token de pareamento nunca aparece aqui.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRequest {
    /// Identificador do pedido (alvo de [`ServerEvent::ApprovalResolved`]
    /// e de `ServerHandle::decide`).
    pub request_id: u64,
    /// ID do dispositivo que pede pareamento.
    pub device_id: String,
    /// Nome informado no `HELLO`.
    pub name: String,
    /// Modelo informado no `HELLO`.
    pub model: String,
    /// IP de onde veio o pareamento.
    pub ip: String,
}

/// Decisão da UI sobre um pedido de aprovação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// "Permitir sempre": cadastra o dispositivo e emite `deviceKey`.
    ApproveAlways,
    /// "Permitir só desta vez": aprova sem `deviceKey`.
    ApproveOnce,
    /// Recusa (`not_approved`).
    Reject,
}

/// Desfecho de um pedido de aprovação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalOutcome {
    /// A UI aprovou (e como).
    Approved(Decision),
    /// A UI recusou.
    Rejected,
    /// Estourou o prazo de aprovação (padrão 30 s).
    TimedOut,
    /// O celular desconectou durante o pedido.
    Cancelled,
}

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
    /// Limitação (F1-T15): taxa de conexões do IP estourada ou IP
    /// bloqueado por falhas de autenticação; fechado logo após o
    /// accept, **sem** resposta.
    RateLimited,
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
    /// Socket de áudio aberto (ticket válido; `WELCOME` entregue).
    AudioOpened { session_id: String },
    /// Socket de áudio fechado (a queda **não** afeta a sessão
    /// principal; o `reason` é o do socket de áudio).
    AudioClosed {
        session_id: String,
        reason: CloseReason,
    },
    /// Novo dispositivo pediu pareamento; aguardando a decisão da UI.
    ApprovalRequested(ApprovalRequest),
    /// Pedido de aprovação resolvido (aprovado, recusado, timeout ou
    /// cancelado por desconexão do celular).
    ApprovalResolved {
        request_id: u64,
        outcome: ApprovalOutcome,
    },
    /// Janela de aprovação USB aberta (F1-T07): o primeiro pareamento
    /// válido de localhost dentro de 15 s é aprovado sem diálogo.
    UsbWindowOpened,
    /// Janela de aprovação USB encerrada: `used = true` se foi
    /// consumida por um pareamento, `false` se expirou.
    UsbWindowClosed { used: bool },
    /// Um IP entrou em bloqueio por falhas de autenticação repetidas
    /// (F1-T15). Sem segredos: nunca o token tentado.
    AuthBlocked { ip: String },
}
