use lc_protocol::json::{Pending, Reason};
use lc_protocol::Message;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

use super::{write_msg, ServerCtx};
use crate::events::{ApprovalOutcome, ApprovalRequest, Decision, EventSink, ServerEvent};
use crate::trust::{ApprovalInfo, Busy};
/// Resultado da aplicação da decisão de aprovação.
pub(super) enum ApplyDecision {
    /// Aprovado; `device_key` só em "permitir sempre".
    Ok { device_key: Option<String> },
    /// Recusar com `reason`.
    Fail(Reason),
}

/// Aplica a decisão de aprovação: cadastra o dispositivo (só
/// "permitir sempre") e rotaciona o token de pareamento.
pub(super) fn apply_pair_decision(
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
pub(super) enum PairApproval {
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
pub(super) async fn pending_approval(
    ctx: &ServerCtx,
    events_tx: &EventSink,
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
            _ = events_tx.shutdown() => break PairApproval::Gone,
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
    events_tx: &EventSink,
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
