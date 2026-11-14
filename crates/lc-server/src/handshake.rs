//! Handshake `HELLO` → `WELCOME`: versão, autenticação (`pair`/`device`),
//! sessão única e emissão de `sessionId`/`audioTicket`.
//!
//! Pareamento (`pair`) com token válido exige aprovação: `auto_approve`
//! (só em build de debug) aprova sempre; senão o servidor envia
//! `PENDING`, emite `ApprovalRequested` e espera decisão da UI, timeout
//! (`approval_timeout`) ou o celular fechar a conexão (cancela o pedido).
//! Antes do pedido de aprovação, uma janela USB aberta (15 s, só
//! loopback, uso único — F1-T07) aprova com `ApproveAlways` sem diálogo.
//!
//! Segredos (`audio_ticket`, `device_key`) existem só na memória e nunca
//! em logs/eventos: o `Debug` de [`Outcome`] os redige.

use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use lc_protocol::json::{Caps, ProtocolVersion, Reason};
use lc_protocol::Message;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, watch};

use crate::audio::TicketStore;
use crate::events::{CloseReason, EventSink, ServerEvent};
use crate::frames::VideoFrameQueue;
use crate::sessions::SessionRegistry;
use crate::trust::{ApprovalManager, Limiters, TrustStore, UsbWindow};

mod approval;
mod auth;
mod perform;
/// Dados compartilhados pelas conexões do servidor.
#[derive(Clone)]
pub struct ServerCtx {
    /// Loja de confiança (token de pareamento e dispositivos).
    pub trust: Arc<Mutex<TrustStore>>,
    /// Tickets de áudio de uso único.
    pub tickets: Arc<Mutex<TicketStore>>,
    /// Registro da sessão ativa (uma única).
    pub sessions: Arc<SessionRegistry>,
    /// Fonte limitada de quadros para consumidores desktop.
    pub(crate) video_frames: Arc<VideoFrameQueue>,
    /// Pedidos de aprovação de pareamento (máximo de um pendente).
    pub approvals: Arc<ApprovalManager>,
    /// Prazo da aprovação no PC (padrão 30 s).
    pub approval_timeout: Duration,
    /// Flag de aprovação automática — só vale em build de debug
    /// (`effective_auto_approve`); em release é ignorada com aviso.
    pub auto_approve: bool,
    /// Janela de aprovação USB (15 s, só loopback, uso único — F1-T07).
    pub usb_window: Arc<Mutex<UsbWindow>>,
    /// Limitadores de segurança (falhas de auth e taxa de conexões —
    /// F1-T15); compartilhados pelo accept loop e pelo handshake.
    pub limiters: Arc<Limiters>,
    /// Identificador estável do PC (UUID do `trust.json`).
    pub pc_id: String,
    /// Nome do PC (mostrado no `WELCOME`).
    pub pc_name: String,
    /// Versão do app (mostrada no `WELCOME`).
    pub app_version: String,
}

/// Hora atual do servidor, em segundos Unix (tickets e `last_seen`).
pub(crate) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Resultado do handshake.
pub enum Outcome {
    /// Aceito: enviar `WELCOME{accepted:true}`; conexão → `Active`.
    Accepted(Box<Accepted>),
    /// Recusado: enviar `WELCOME{accepted:false}` e fechar.
    Denied {
        reason: Reason,
        min_supported: Option<ProtocolVersion>,
    },
    /// Fechar sem responder (primeiro quadro fora de ordem).
    Close(CloseReason),
}

/// Dados de [`Outcome::Accepted`].
///
/// `audio_ticket` e `device_key` são **segredos** (redigidos no
/// `Debug`): jamais em logs ou eventos.
pub struct Accepted {
    pub session_id: String,
    pub audio_ticket: String,
    pub device_key: Option<String>,
    pub device_id: String,
    pub device_name: String,
    pub caps: Caps,
    /// Dispara se **esta** sessão for substituída pelo mesmo dispositivo
    /// (a conexão envia `BYE{reason:"replaced"}`).
    pub kick_rx: oneshot::Receiver<()>,
    /// Canal PC→celular da sessão; o laço ativo serializa todas as escritas.
    pub command_rx: mpsc::Receiver<Message>,
}

impl fmt::Debug for Accepted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Accepted")
            .field("session_id", &self.session_id)
            .field("device_id", &self.device_id)
            .field("audio_ticket", &"<redigido>")
            .field("device_key", &"<redigida>")
            .finish()
    }
}

impl fmt::Debug for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accepted(data) => data.fmt(f),
            Self::Denied {
                reason,
                min_supported,
            } => f
                .debug_struct("Denied")
                .field("reason", reason)
                .field("min_supported", min_supported)
                .finish(),
            Self::Close(reason) => f.debug_tuple("Close").field(reason).finish(),
        }
    }
}

/// Executa o handshake `HELLO` → `WELCOME` sobre o primeiro quadro.
///
/// Ordem: versão → autenticação (`pair`/`device`) → sessão única.
/// `events_tx`/`stream`/`buf` são usados só no fluxo de aprovação
/// pendente (`PENDING` + leitura para detectar o celular sair).
/// Nunca `unwrap()`s; erros de persistência degradam para recusa.
/// Executa o handshake usando o canal público de eventos.
/// Mantém a assinatura anterior; o servidor usa `perform_with_sink` para
/// tornar as emissões canceláveis pelo shutdown.
pub async fn perform(
    ctx: &ServerCtx,
    events_tx: &mpsc::Sender<ServerEvent>,
    stream: &mut TcpStream,
    buf: &mut [u8],
    conn_id: u64,
    first: &Message,
    peer: SocketAddr,
) -> Outcome {
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);
    let sink = EventSink::new(events_tx.clone(), shutdown_rx);
    perform_with_sink(ctx, &sink, stream, buf, conn_id, first, peer).await
}

/// Variante interna que observa o shutdown compartilhado pelo servidor.
pub(crate) async fn perform_with_sink(
    ctx: &ServerCtx,
    events_tx: &EventSink,
    stream: &mut TcpStream,
    buf: &mut [u8],
    conn_id: u64,
    first: &Message,
    peer: SocketAddr,
) -> Outcome {
    perform::perform_with_sink(ctx, events_tx, stream, buf, conn_id, first, peer).await
}
/// Serializa e envia a mensagem (flush no fim). Compartilhado com
/// `conn.rs` (que importa esta função).
pub(crate) async fn write_msg<W>(stream: &mut W, msg: &Message) -> io::Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let bytes = msg
        .encode(0)
        .map_err(|e| io::Error::other(format!("encode de quadro: {e}")))?;
    stream.write_all(&bytes).await?;
    stream.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UT: `Outcome::Debug` nunca expõe os segredos.
    #[test]
    fn outcome_debug_redacts_secrets() {
        let outcome = Outcome::Accepted(Box::new(Accepted {
            session_id: "ab".repeat(16),
            audio_ticket: "secret-ticket-0123456789abcdef".to_owned(),
            device_key: Some("secret-key-0123456789abcdef".to_owned()),
            device_id: "dev-1".to_owned(),
            device_name: "Fone".to_owned(),
            caps: Caps::default(),
            kick_rx: oneshot::channel::<()>().1,
            command_rx: mpsc::channel(1).1,
        }));
        let rendered = format!("{outcome:?}");
        assert!(!rendered.contains("secret-ticket"), "{rendered}");
        assert!(!rendered.contains("secret-key"), "{rendered}");
        assert!(rendered.contains("dev-1"), "{rendered}");
    }
}
