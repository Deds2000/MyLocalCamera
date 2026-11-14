use std::net::SocketAddr;

use lc_protocol::json::{Auth, Hello, Reason};
use lc_protocol::Message;
use tokio::net::TcpStream;

use super::approval::{apply_pair_decision, pending_approval, ApplyDecision, PairApproval};
use super::auth::{device_auth, note_auth_failure, DeviceAuth};
use super::{now_secs, Accepted, Outcome, ServerCtx};
use crate::events::{CloseReason, Decision, EventSink, ServerEvent};
use crate::sessions::Claim;
use crate::trust::{effective_auto_approve, secrets};
use crate::version;
/// Variante interna que observa o shutdown compartilhado pelo servidor.
pub(super) async fn perform_with_sink(
    ctx: &ServerCtx,
    events_tx: &EventSink,
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
        // Defesa em profundidade (F1-T09b): o socket de áudio é
        // desviado em `conn.rs` antes do handshake; se um `AUDIO_HELLO`
        // ainda chegar aqui, recusar com `bad_audio_ticket`.
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
    events_tx: &EventSink,
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
                let mut usb = ctx.usb_window.lock().unwrap_or_else(|p| p.into_inner());
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
    match ctx
        .sessions
        .claim(session_id.clone(), &hello.device.id, conn_id)
    {
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
