//! Tipos públicos do `mock-sender`: configuração, resultado e erros.
//!
//! Extraídos de `session.rs` para manter os arquivos dentro do limite de
//! ~300 linhas; `session` reexporta tudo, então a API não muda.

use std::path::PathBuf;

use lc_protocol::json::Reason;

use crate::faults::Faults;

/// Configuração resolvida da execução.
#[derive(Debug, Clone)]
pub struct MockConfig {
    pub hosts: Vec<String>,
    pub port: u16,
    pub token: Option<String>,
    pub device_id: String,
    pub device_name: String,
    pub device_key: Option<String>,
    pub state_file: Option<PathBuf>,
    pub file: Option<PathBuf>,
    pub loop_video: bool,
    pub fps: u32,
    pub audio: bool,
    pub tone: f32,
    pub seconds: Option<f64>,
    /// Modos de falha determinísticos (F1-T11b).
    pub faults: Faults,
}

/// Resultado de uma execução (agregado quando `--reconnect N`).
#[derive(Debug, Clone, Default)]
pub struct Summary {
    pub video_frames: u64,
    pub video_bytes: u64,
    pub audio_blocks: u64,
    /// Sessões concluídas (`--reconnect N`; 1 nas demais).
    pub sessions: u32,
    /// `Some("server")` nos quadros malformados; `Some("killed")` no
    /// `--kill-after`; `Some(reason)` quando veio `BYE`.
    pub closed_by: Option<String>,
    /// Tempo até o servidor fechar (modos malformados).
    pub closed_after_ms: Option<u64>,
    pub session_id: String,
}

/// Falhas do mock.
#[derive(Debug, thiserror::Error)]
pub enum MockError {
    /// Servidor recusou o pareamento/autenticação.
    #[error("servidor recusou: {0:?}")]
    Denied(Option<Reason>),
    /// Nenhum host da lista aceitou a conexão.
    #[error("nenhum host respondeu ({0})")]
    NoHost(String),
    /// Conexão caiu antes do `WELCOME`.
    #[error("conexão encerrada sem WELCOME")]
    Closed,
    /// `WELCOME` aceito sem `sessionId`.
    #[error("WELCOME sem sessionId")]
    MissingSession,
    /// Áudio pedido, mas o `WELCOME` não trouxe `audioTicket`.
    #[error("WELCOME sem audioTicket")]
    MissingAudioTicket,
    /// Configuração incompleta (sem token nem `deviceKey`).
    #[error("configuração: {0}")]
    Config(String),
    /// Arquivo de vídeo inválido.
    #[error("vídeo: {0}")]
    Video(String),
    /// Erro de rede/IO.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl MockError {
    /// Código de saída do binário: `2` para recusa do servidor, `1` para
    /// os demais erros (0 = sucesso).
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Denied(_) => 2,
            _ => 1,
        }
    }
}
