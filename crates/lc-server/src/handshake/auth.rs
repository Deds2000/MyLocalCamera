use std::net::IpAddr;

use lc_protocol::json::Reason;

use super::ServerCtx;
use crate::events::{EventSink, ServerEvent};
use crate::trust::VerifyDevice;
/// Resultado da autenticação `device`.
pub(super) enum DeviceAuth {
    /// Cadastrado e a chave confere.
    Ok,
    /// Recusar com `reason`.
    Fail(Reason),
}

pub(super) fn device_auth(
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
pub(super) async fn note_auth_failure(
    ctx: &ServerCtx,
    events_tx: &EventSink,
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
            .send(ServerEvent::AuthBlocked { ip: ip.to_string() })
            .await;
    }
}
