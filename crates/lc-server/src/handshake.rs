//! Handshake `HELLO` → `WELCOME`: versão, autenticação (`pair`/`device`),
//! sessão única e emissão de `sessionId`/`audioTicket`.
//!
//! A **decisão de aprovação é um stub** (resolvido no F1-T06): se
//! `auto_approve` está ligado, aprova sempre; senão recusa.
//!
//! Segredos (`audio_ticket`, `device_key`) existem só na memória e nunca
//! em logs/eventos: o `Debug` de [`Outcome`] os redige.

use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use lc_protocol::json::{Auth, Caps, Hello, ProtocolVersion, Reason};
use lc_protocol::Message;
use tokio::sync::oneshot;

use crate::audio::TicketStore;
use crate::events::CloseReason;
use crate::sessions::{Claim, SessionRegistry};
use crate::trust::{secrets, TrustStore, VerifyDevice};
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
    /// Aprovação automática (stub — F1-T06): `true` = "permitir sempre".
    pub auto_approve: bool,
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

/// Decisão de aprovação de um novo pareamento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// "Permitir sempre": cadastra o dispositivo e emite `deviceKey`.
    ApproveAlways,
    /// "Permitir só desta vez": aprova sem `deviceKey`.
    ApproveOnce,
    /// Recusa (`not_approved`).
    Reject,
}

/// Decisão de aprovação — **stub** (o diálogo real chega no F1-T06):
/// `auto_approve` liga → aprova sempre; desliga → recusa.
#[must_use]
pub fn decide_approval(auto_approve: bool) -> Decision {
    if auto_approve {
        Decision::ApproveAlways
    } else {
        Decision::Reject
    }
}

/// Executa o handshake `HELLO` → `WELCOME` sobre o primeiro quadro.
///
/// Ordem: versão → autenticação (`pair`/`device`) → sessão única.
/// Nunca `unwrap()`s; erros de persistência degradam para recusa.
pub async fn perform(ctx: &ServerCtx, conn_id: u64, first: &Message, peer: SocketAddr) -> Outcome {
    match first {
        Message::Hello(hello) => perform_hello(ctx, conn_id, hello, peer),
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

fn perform_hello(
    ctx: &ServerCtx,
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
        Auth::Pair { token } => match pair_auth(ctx, token, &hello.device.id, &hello.device.name, &ip, now) {
            PairAuth::Ok { device_key } => device_key,
            PairAuth::Fail(reason) => {
                return Outcome::Denied {
                    reason,
                    min_supported: None,
                }
            }
        },
        Auth::Device { device_id, key } => match device_auth(ctx, device_id, key, &ip, now) {
            DeviceAuth::Ok => None, // reconexão: WELCOME sem `deviceKey`
            DeviceAuth::Fail(reason) => {
                return Outcome::Denied {
                    reason,
                    min_supported: None,
                }
            }
        },
    };

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

/// Resultado da autenticação `pair`.
enum PairAuth {
    /// Token válido e aprovado; `device_key` só em "permitir sempre".
    Ok { device_key: Option<String> },
    /// Recusar com `reason`.
    Fail(Reason),
}

fn pair_auth(
    ctx: &ServerCtx,
    token: &str,
    device_id: &str,
    device_name: &str,
    ip: &str,
    now: u64,
) -> PairAuth {
    let mut trust = ctx.trust.lock().unwrap_or_else(|p| p.into_inner());
    // `verify_pair_token` compara em tempo constante.
    if !trust.verify_pair_token(token) {
        // Falha de auth registrada para o limitador do F1-T15.
        tracing::debug!(device_id, "pareamento recusado: token inválido");
        return PairAuth::Fail(Reason::BadToken);
    }

    // Decisão de aprovação — **stub** (F1-T06).
    let device_key = match decide_approval(ctx.auto_approve) {
        Decision::Reject => return PairAuth::Fail(Reason::NotApproved),
        Decision::ApproveOnce => None,
        Decision::ApproveAlways => match trust.add_device(device_id, device_name, ip, now) {
            Ok(key) => Some(key),
            Err(e) => {
                tracing::error!(error = %e, "falha ao cadastrar dispositivo confiável");
                // Dispositivo não ficou cadastrado → "não aprovado".
                return PairAuth::Fail(Reason::NotApproved);
            }
        },
    };

    // Rotaciona o token de pareamento (nas duas aprovações). Se falhar,
    // a sessão segue; o token rotaciona no próximo pareamento.
    if let Err(e) = trust.regenerate_token() {
        tracing::error!(error = %e, "falha ao rotacionar token de pareamento");
    }
    PairAuth::Ok { device_key }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// UT: o stub aprova sempre com `auto_approve` e recusa sem ele.
    #[test]
    fn approval_stub_follows_config() {
        assert_eq!(decide_approval(true), Decision::ApproveAlways);
        assert_eq!(decide_approval(false), Decision::Reject);
    }

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
