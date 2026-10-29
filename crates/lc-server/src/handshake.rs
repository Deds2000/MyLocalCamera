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
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use lc_protocol::json::{Auth, Caps, Hello, Pending, ProtocolVersion, Reason};
use lc_protocol::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

use crate::audio::TicketStore;
use crate::events::{
    ApprovalOutcome, ApprovalRequest, CloseReason, Decision, ServerEvent,
};
use crate::sessions::{Claim, SessionRegistry};
use crate::trust::{
    effective_auto_approve, secrets, ApprovalInfo, ApprovalManager, Busy, Limiters, TrustStore,
    UsbWindow, VerifyDevice,
};
use crate::version;

/// Dados compartilhados pelas conexões do servidor.
#[derive(Clone)]
pub struct ServerCtx {
    /// Loja de confiança (token de pareamento e dispositivos).
    pub trust: Arc<Mutex<TrustStore>>,
    /// Tickets de áudio de uso único.
    pub tickets: Arc<Mutex<TicketStore>>,
    /// Registro da sessão ativa (uma única).
    pub sessions: Arc<SessionRegistry>,
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
            } => f.debug_struct("Denied")
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
pub async fn perform(
    ctx: &ServerCtx,
    events_tx: &mpsc::Sender<ServerEvent>,
    stream: &mut TcpStream,
    buf: &mut [u8],
    conn_id: u64,
    first: &Message,
    peer: SocketAddr,
) -> Outcome {
    match first {
        Message::Hello(hello) => {
            perform_hello(ctx, events_tx, stream, buf, conn_id, hello, peer).await
        }
        // `AUDIO_HELLO` no socket principal é papel do socket de áudio
        // (F1-T09b): recusar com `bad_audio_ticket`.
        Message::AudioHello(_) => Outcome::Denied {
            reason: Reason::BadAudioTicket,
            min_supported: None,
        },
        // Qualquer outro primeiro quadro é fora de ordem: fechar calado.
        _ => Outcome::Close(CloseReason::ProtocolError),
    }
}

async fn perform_hello(
    ctx: &ServerCtx,
    events_tx: &mpsc::Sender<ServerEvent>,
    stream: &mut TcpStream,
    buf: &mut [u8],
    conn_id: u64,
    hello: &Hello,
    peer: SocketAddr,
) -> Outcome {
    // 1) Versão: `major` deve bater; `minor` diferente é aceito.
    if let Err(min) = version::negotiate(&hello.protocol) {
        return Outcome::Denied {
            reason: Reason::VersionMismatch,
            min_supported: Some(min),
        };
    }
    let now = now_secs();
    let ip = peer.ip().to_string();

    // 2) Autenticação (o `Denied` carrega `min_supported: None`).
    let device_key = match &hello.auth {
        Auth::Pair { token } => {
            // 2.1) Token (comparação em tempo constante na loja).
            let token_ok = {
                let trust = ctx.trust.lock().unwrap_or_else(|p| p.into_inner());
                trust.verify_pair_token(token)
            };
            if !token_ok {
                // Falha contada no limitador (F1-T15); o token tentado
                // nunca é logado e a resposta segue uniforme.
                note_auth_failure(ctx, events_tx, peer.ip(), now).await;
                tracing::debug!(device_id = %hello.device.id, "pareamento recusado: token inválido");
                return Outcome::Denied {
                    reason: Reason::BadToken,
                    min_supported: None,
                };
            }

            // 2.2) Janela de aprovação USB (15 s, loopback, uso único —
            //     F1-T07): o primeiro `HELLO` válido de localhost a
            //     consome e é aprovado sem diálogo. Token inválido não
            //     chega aqui (recusado em 2.1) e não consome a janela.
            let (usb_expired, usb_consumed) = {
                let mut usb = ctx
                    .usb_window
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                let expired = usb.expire_if_stale(now);
                let consumed = usb.try_consume(peer.ip(), now);
                (expired, consumed)
            };
            if usb_expired {
                let _ = events_tx
                    .send(ServerEvent::UsbWindowClosed { used: false })
                    .await;
            }
            if usb_consumed {
                let _ = events_tx
                    .send(ServerEvent::UsbWindowClosed { used: true })
                    .await;
            }

            // 2.3) Decisão de aprovação: janela USB consumida ou
            //     `auto_approve` (só em build de debug) aprovam direto.
            let decision = if usb_consumed
                || effective_auto_approve(ctx.auto_approve, cfg!(debug_assertions))
            {
                Decision::ApproveAlways
            } else {
                if ctx.auto_approve {
                    tracing::warn!("auto_approve é ignorado fora de build de debug");
                }
                match pending_approval(ctx, events_tx, stream, buf, conn_id, &hello.device, &ip)
                    .await
                {
                    PairApproval::Decided(d) => d,
                    PairApproval::Busy => {
                        return Outcome::Denied {
                            reason: Reason::Busy,
                            min_supported: None,
                        };
                    }
                    PairApproval::TimedOut => {
                        tracing::info!(device_id = %hello.device.id, "aprovação expirou (timeout)");
                        return Outcome::Denied {
                            reason: Reason::NotApproved,
                            min_supported: None,
                        };
                    }
                    PairApproval::Gone => {
                        // pedido cancelado e
                        // `ApprovalResolved{Cancelled}` emitidos em
                        // `pending_approval`; o par já foi embora
                        return Outcome::Close(CloseReason::PeerClosed);
                    }
                }
            };

            // 2.4) Aplica a decisão (registro + rotação do token).
            match apply_pair_decision(ctx, decision, &hello.device, &ip, now) {
                ApplyDecision::Ok { device_key } => device_key,
                ApplyDecision::Fail(reason) => {
                    return Outcome::Denied {
                        reason,
                        min_supported: None,
                    };
                }
            }
        }
        Auth::Device { device_id, key } => match device_auth(ctx, device_id, key, &ip, now) {
            DeviceAuth::Ok => None, // reconexão: WELCOME sem `deviceKey`
            DeviceAuth::Fail(Reason::BadToken) => {
                // `BadKey`: conta como falha de auth (F1-T15);
                // `revoked` **não** conta.
                note_auth_failure(ctx, events_tx, peer.ip(), now).await;
                return Outcome::Denied {
                    reason: Reason::BadToken,
                    min_supported: None,
                };
            }
            DeviceAuth::Fail(reason) => {
                return Outcome::Denied {
                    reason,
                    min_supported: None,
                }
            }
        },
    };

    // Auth OK: zera as falhas anteriores deste IP (F1-T15).
    ctx.limiters
        .auth
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .record_success(peer.ip());

    // 3) Sessão única: sessão ativa de outro dispositivo → `busy`; do
    //    mesmo dispositivo → substituição (BYE para a antiga, lá em `conn`).
    let session_id = secrets::random_hex(16);
    match ctx.sessions.claim(session_id.clone(), &hello.device.id, conn_id) {
        Claim::Busy => Outcome::Denied {
            reason: Reason::Busy,
            min_supported: None,
        },
        Claim::Granted { kick_rx, .. } => {
            let audio_ticket = ctx
                .tickets
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .issue(&session_id, now);
            Outcome::Accepted(Box::new(Accepted {
                session_id,
                audio_ticket,
                device_key,
                device_id: hello.device.id.clone(),
                device_name: hello.device.name.clone(),
                caps: hello.caps.clone(),
                kick_rx,
            }))
        }
    }
}

/// Resultado da aplicação da decisão de aprovação.
enum ApplyDecision {
    /// Aprovado; `device_key` só em "permitir sempre".
    Ok { device_key: Option<String> },
    /// Recusar com `reason`.
    Fail(Reason),
}

/// Aplica a decisão de aprovação: cadastra o dispositivo (só
/// "permitir sempre") e rotaciona o token de pareamento.
fn apply_pair_decision(
    ctx: &ServerCtx,
    decision: Decision,
    device: &lc_protocol::json::DeviceInfo,
    ip: &str,
    now: u64,
) -> ApplyDecision {
    let mut trust = ctx.trust.lock().unwrap_or_else(|p| p.into_inner());
    let device_key = match decision {
        Decision::ApproveAlways => match trust.add_device(&device.id, &device.name, ip, now) {
            Ok(key) => Some(key),
            Err(e) => {
                tracing::error!(error = %e, "falha ao cadastrar dispositivo confiável");
                // Dispositivo não ficou cadastrado → "não aprovado".
                return ApplyDecision::Fail(Reason::NotApproved);
            }
        },
        Decision::ApproveOnce => None,
        Decision::Reject => return ApplyDecision::Fail(Reason::NotApproved),
    };

    // Rotaciona o token de pareamento (nas duas aprovações). Se falhar,
    // a sessão segue; o token rotaciona no próximo pareamento.
    if let Err(e) = trust.regenerate_token() {
        tracing::error!(error = %e, "falha ao rotacionar token de pareamento");
    }
    ApplyDecision::Ok { device_key }
}

/// Desfecho da espera pela decisão de aprovação.
enum PairApproval {
    /// Decisão da UI (`decide` já removeu o pedido pendente).
    Decided(Decision),
    /// Estourou o prazo de aprovação (`not_approved`).
    TimedOut,
    /// Celular desconectou (ou a conexão quebrou) antes da decisão.
    Gone,
    /// Já havia um pedido de aprovação pendente (`busy`).
    Busy,
}

/// Abre o pedido de aprovação (máximo de um pendente), envia
/// `PENDING{timeoutS}` ao celular, emite `ApprovalRequested` e espera:
/// decisão da UI, timeout (`approval_timeout`) ou o celular fechar a
/// conexão. Sempre termina emitindo `ApprovalResolved` e removendo o
/// pedido pendente.
async fn pending_approval(
    ctx: &ServerCtx,
    events_tx: &mpsc::Sender<ServerEvent>,
    stream: &mut TcpStream,
    buf: &mut [u8],
    conn_id: u64,
    device: &lc_protocol::json::DeviceInfo,
    ip: &str,
) -> PairApproval {
    let info = ApprovalInfo {
        device_id: device.id.clone(),
        name: device.name.clone(),
        model: device.model.clone(),
        ip: ip.to_owned(),
    };
    let (request_id, mut rx) = match ctx.approvals.begin(&info) {
        Err(Busy) => {
            // Um pedido por vez: outro pareamento durante o pendente.
            tracing::debug!(device_id = %device.id, "pareamento recusado: pedido pendente em andamento");
            return PairApproval::Busy;
        }
        Ok(v) => v,
    };

    let timeout_s = ctx.approval_timeout.as_secs().max(1) as u32;
    let pending = Message::Pending(Pending { timeout_s });
    if let Err(e) = write_msg(stream, &pending).await {
        // O par já não está mais aqui: cancela o pedido e fecha.
        cancel_resolved(ctx, events_tx, request_id, ApprovalOutcome::Cancelled).await;
        tracing::warn!(conn_id, error = %e, "falha ao enviar PENDING; encerrando");
        return PairApproval::Gone;
    }

    let _ = events_tx
        .send(ServerEvent::ApprovalRequested(ApprovalRequest {
            request_id,
            device_id: device.id.clone(),
            name: device.name.clone(),
            model: device.model.clone(),
            ip: ip.to_owned(),
        }))
        .await;

    let deadline = tokio::time::sleep(ctx.approval_timeout);
    tokio::pin!(deadline);
    let wait = loop {
        tokio::select! {
            // o `Receiver` é o próprio `Future` (tokio::sync::oneshot)
            res = &mut rx => break match res {
                Ok(d) => PairApproval::Decided(d),
                // `tx` caiu sem decisão (defensivo; o normal é `Gone`
                // pela leitura abaixo).
                Err(_) => PairApproval::Gone,
            },
            _ = &mut deadline => break match rx.try_recv() {
                // Decisão chegou junto com o timeout: vale a decisão.
                Ok(d) => PairApproval::Decided(d),
                Err(_) => PairApproval::TimedOut,
            },
            res = stream.read(buf) => match res {
                Ok(0) => break PairApproval::Gone, // celular fechou
                Ok(n) => {
                    // O celular não manda nada antes do WELCOME;
                    // consome e segue esperando.
                    tracing::debug!(conn_id, bytes = n, "dados durante pedido pendente; ignorando");
                }
                Err(e) => {
                    tracing::warn!(conn_id, error = %e, "erro de leitura durante pedido pendente");
                    break PairApproval::Gone;
                }
            },
        }
    };

    let outcome = match &wait {
        PairApproval::Decided(d) => match d {
            Decision::Reject => ApprovalOutcome::Rejected,
            other => ApprovalOutcome::Approved(*other),
        },
        PairApproval::TimedOut => ApprovalOutcome::TimedOut,
        // `Busy` não chega aqui (retorna antes do wait); `Cancelled`
        // é o desfecho mais conservador.
        PairApproval::Gone | PairApproval::Busy => ApprovalOutcome::Cancelled,
    };
    cancel_resolved(ctx, events_tx, request_id, outcome).await;
    wait
}

/// Emite `ApprovalResolved` e libera o pedido pendente (no-op se a
/// decisão da UI já o removeu).
async fn cancel_resolved(
    ctx: &ServerCtx,
    events_tx: &mpsc::Sender<ServerEvent>,
    request_id: u64,
    outcome: ApprovalOutcome,
) {
    let _ = events_tx
        .send(ServerEvent::ApprovalResolved {
            request_id,
            outcome,
        })
        .await;
    ctx.approvals.cancel(request_id);
}

/// Resultado da autenticação `device`.
enum DeviceAuth {
    /// Cadastrado e a chave confere.
    Ok,
    /// Recusar com `reason`.
    Fail(Reason),
}

fn device_auth(
    ctx: &ServerCtx,
    device_id: &str,
    key: &str,
    ip: &str,
    now: u64,
) -> DeviceAuth {
    let mut trust = ctx.trust.lock().unwrap_or_else(|p| p.into_inner());
    match trust.verify_device(device_id, key) {
        VerifyDevice::Ok => {
            // Atualiza `last_seen`/`last_ip` (falha de I/O não derruba).
            if let Err(e) = trust.touch(device_id, ip, now) {
                tracing::warn!(error = %e, "falha ao atualizar last_seen");
            }
            DeviceAuth::Ok
        }
        VerifyDevice::Unknown => DeviceAuth::Fail(Reason::Revoked),
        VerifyDevice::BadKey => DeviceAuth::Fail(Reason::BadToken),
    }
}

/// Registra uma falha de autenticação no limitador (F1-T15) e emite
/// [`ServerEvent::AuthBlocked`] se o IP **acabou de entrar** em
/// bloqueio. Sem segredos: nunca o token tentado.
async fn note_auth_failure(
    ctx: &ServerCtx,
    events_tx: &mpsc::Sender<ServerEvent>,
    ip: IpAddr,
    now: u64,
) {
    let just_blocked = ctx
        .limiters
        .auth
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .record_failure(ip, now);
    if just_blocked {
        let _ = events_tx
            .send(ServerEvent::AuthBlocked {
                ip: ip.to_string(),
            })
            .await;
    }
}

/// Serializa e envia a mensagem (flush no fim). Compartilhado com
/// `conn.rs` (que importa esta função).
pub(crate) async fn write_msg(stream: &mut TcpStream, msg: &Message) -> io::Result<()> {
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
        }));
        let rendered = format!("{outcome:?}");
        assert!(!rendered.contains("secret-ticket"), "{rendered}");
        assert!(!rendered.contains("secret-key"), "{rendered}");
        assert!(rendered.contains("dev-1"), "{rendered}");
    }
}
