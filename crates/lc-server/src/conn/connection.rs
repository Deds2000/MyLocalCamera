use std::net::SocketAddr;
use std::time::Duration;

use lc_protocol::{type_name, Frame, FrameReader, Message};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::{oneshot, watch, OwnedSemaphorePermit};
use tracing::Instrument;

use super::accept::{ConnShared, KeepaliveTiming};
use super::session::{
    accepted_welcome, denied_welcome, emit_closed, purge_ticket, session_loop, step, SessionData,
};
use crate::audio::conn::{run_audio, AudioConn};
use crate::events::{CloseReason, ServerEvent};
use crate::handshake::{self, write_msg, Accepted, Outcome};
use crate::redact::short;
use crate::state::ConnState;

/// Tamanho do buffer de leitura por conexão.
const READ_BUF_SIZE: usize = 64 * 1024;
/// Laço por conexão:
/// 1. 1º quadro (`AwaitHello`, com prazo): `HELLO` → handshake;
///    `AUDIO_HELLO` → socket de áudio (F1-T09b); qualquer outra coisa
///    → `ProtocolError` (sem resposta);
/// 2. `WELCOME` enviado — aceito: libera o permit e entra na sessão;
///    negado: fecha **após** enviar;
/// 3. laço de sessão (`Active`): `PING`→`PONG`, `AUDIO_TICKET_REQUEST`
///    →`AUDIO_TICKET`, `BYE`→encerra.
pub(super) async fn run_conn(
    mut stream: TcpStream,
    peer: SocketAddr,
    conn_id: u64,
    permit: OwnedSemaphorePermit,
    shared: ConnShared,
) {
    let ConnShared {
        ctx,
        events_tx,
        mut shutdown_rx,
        hello_timeout,
        audio_block_max,
        timing,
    } = shared;
    let KeepaliveTiming {
        ping_interval,
        liveness_timeout,
        stats_interval,
    } = timing;
    if let Err(e) = stream.set_nodelay(true) {
        tracing::warn!(conn_id, %peer, error = %e, "TCP_NODELAY falhou");
        emit_closed(&events_tx, conn_id, CloseReason::ConnectionLost).await;
        return;
    }

    let mut reader = FrameReader::new();
    let mut buf = [0u8; READ_BUF_SIZE];

    // ── 1) Primeiro quadro (AwaitHello) ──
    let first = read_first_frame(
        &mut stream,
        &mut reader,
        &mut buf,
        &mut shutdown_rx,
        hello_timeout,
    )
    .await;
    let msg = match first {
        FirstFrame::Closed(reason) => {
            emit_closed(&events_tx, conn_id, reason).await;
            return;
        }
        FirstFrame::Frame(frame) => {
            tracing::debug!(
                conn_id,
                %peer,
                frame_type = type_name(frame.msg_type),
                payload_len = frame.payload.len(),
                "primeiro frame recebido"
            );
            match Message::from_frame(&frame) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(conn_id, %peer, error = %e, "primeiro frame inválido; fechando");
                    emit_closed(&events_tx, conn_id, CloseReason::ProtocolError).await;
                    return;
                }
            }
        }
    };

    // ── Socket de áudio (F1-T09b): 1º quadro `AUDIO_HELLO` ──
    if let Message::AudioHello(hello) = &msg {
        // Bytes além do `AUDIO_HELLO` no mesmo segmento são fora de
        // ordem (o celular espera o `WELCOME`): fecha calado.
        let reason = if reader.pending_len() > 0 {
            tracing::debug!(conn_id, %peer, "dados antes do WELCOME de áudio; fechando");
            CloseReason::ProtocolError
        } else {
            run_audio(
                AudioConn {
                    ctx: ctx.clone(),
                    events_tx: events_tx.clone(),
                    shutdown_rx: shutdown_rx.clone(),
                    audio_block_max,
                    ping_interval,
                    liveness_timeout,
                },
                &mut stream,
                peer,
                conn_id,
                permit,
                hello,
            )
            .await
        };
        step(ConnState::AwaitHello, ConnState::Closing);
        emit_closed(&events_tx, conn_id, reason).await;
        return;
    }

    let mut state = ConnState::AwaitHello;
    if matches!(msg, Message::Hello(_)) {
        step(state, ConnState::Authenticating);
        state = ConnState::Authenticating;
    }

    // ── 2) Handshake (versão, auth pair/device + aprovação, sessão única) ──
    let outcome =
        handshake::perform_with_sink(&ctx, &events_tx, &mut stream, &mut buf, conn_id, &msg, peer)
            .await;
    match outcome {
        Outcome::Denied {
            reason,
            min_supported,
        } => {
            // fecha **após** enviar o WELCOME (mesmo se o envio falhar)
            let _ = write_msg(
                &mut stream,
                &Message::Welcome(denied_welcome(reason, min_supported)),
            )
            .await;
            step(state, ConnState::Closing);
            emit_closed(&events_tx, conn_id, CloseReason::AuthDenied).await;
        }
        Outcome::Close(reason) => {
            // fora de ordem: fechar calado, sem resposta
            step(state, ConnState::Closing);
            emit_closed(&events_tx, conn_id, reason).await;
        }
        Outcome::Accepted(a) => {
            let Accepted {
                session_id,
                audio_ticket,
                device_key,
                device_id,
                device_name,
                caps,
                kick_rx,
                command_rx,
            } = *a;
            let welcome = Message::Welcome(accepted_welcome(
                &ctx,
                &session_id,
                audio_ticket,
                device_key,
            ));
            if let Err(e) = write_msg(&mut stream, &welcome).await {
                tracing::warn!(conn_id, %peer, error = %e, "falha ao enviar WELCOME; fechando");
                ctx.sessions.release(&session_id);
                purge_ticket(&ctx, &session_id);
                step(state, ConnState::Closing);
                emit_closed(&events_tx, conn_id, CloseReason::ConnectionLost).await;
                return;
            }
            // liberado o permit de não autenticadas
            drop(permit);
            // Span da sessão (F1-T16): logs rastreáveis pelo id abreviado.
            let session_span = tracing::info_span!("session", session = %short(&session_id));
            session_span.in_scope(|| tracing::debug!(device_id = %device_id, "sessão iniciada"));
            step(state, ConnState::Active);
            state = ConnState::Active;
            let _ = events_tx
                .send(ServerEvent::SessionStarted {
                    conn_id,
                    session_id: session_id.clone(),
                    device_id,
                    device_name,
                    peer,
                    caps,
                })
                .await;

            // ── 3) Sessão (Active) ──
            // Contadores da sessão (a sessão recém-aceita é a ativa;
            // `None` só na race de substituição — a sessão encerra via
            // `kick_rx` antes de qualquer estatística).
            let counters = ctx.sessions.counters(&session_id);
            // Task agregadora (F1-T10): emite `Stats` a cada
            // `stats_interval`; termina com a sessão.
            let stats = counters.as_ref().map(|c| {
                let (tx, rx) = oneshot::channel();
                let task = tokio::spawn(crate::stats::session_stats_task(
                    session_id.clone(),
                    c.clone(),
                    stats_interval,
                    events_tx.clone(),
                    rx,
                ));
                (tx, task)
            });
            let mut session_data = SessionData {
                ctx: ctx.clone(),
                session_id: session_id.clone(),
                counters,
                command_rx,
                ping_interval,
                liveness_timeout,
            };
            let reason = session_loop(
                &mut stream,
                &mut reader,
                &mut buf,
                &mut session_data,
                conn_id,
                kick_rx,
                &mut shutdown_rx,
            )
            .instrument(session_span)
            .await;
            // Encerra a task de estatísticas com a sessão.
            if let Some((stats_tx, stats_task)) = stats {
                let _ = stats_tx.send(());
                if let Err(e) = stats_task.await {
                    tracing::error!(conn_id, error = %e, "task de estatísticas panicou");
                }
            }
            let _ = events_tx
                .send(ServerEvent::SessionEnded {
                    session_id: session_id.clone(),
                    reason: reason.clone(),
                })
                .await;
            ctx.sessions.release(&session_id);
            purge_ticket(&ctx, &session_id);
            step(state, ConnState::Closing);
            emit_closed(&events_tx, conn_id, reason).await;
        }
    }
}

/// Resultado da espera pelo primeiro quadro.
enum FirstFrame {
    /// Quadro completo (o 1º consumido; o resto fica no `FrameReader`).
    Frame(Frame),
    /// Conexão terminou antes de um quadro completo.
    Closed(CloseReason),
}

async fn read_first_frame(
    stream: &mut TcpStream,
    reader: &mut FrameReader,
    buf: &mut [u8],
    shutdown_rx: &mut watch::Receiver<bool>,
    hello_timeout: Duration,
) -> FirstFrame {
    let deadline = tokio::time::sleep(hello_timeout);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            res = stream.read(buf) => match res {
                Ok(0) => return FirstFrame::Closed(CloseReason::PeerClosed),
                Ok(n) => match reader.push(&buf[..n]) {
                    Ok(frames) => match frames.len() {
                        0 => {} // quadro incompleto: espera mais bytes
                        1 => return FirstFrame::Frame(frames.into_iter().next().expect("1 quadro")),
                        // mensagens antes do WELCOME são fora de ordem
                        _ => return FirstFrame::Closed(CloseReason::ProtocolError),
                    },
                    Err(e) => {
                        tracing::warn!(error = %e, "erro de protocolo antes do WELCOME");
                        return FirstFrame::Closed(CloseReason::ProtocolError);
                    }
                },
                Err(e) => {
                    tracing::warn!(error = %e, "erro de leitura");
                    return FirstFrame::Closed(CloseReason::ConnectionLost);
                }
            },
            _ = &mut deadline => return FirstFrame::Closed(CloseReason::HelloTimeout),
            _ = shutdown_rx.changed() => return FirstFrame::Closed(CloseReason::Shutdown),
        }
    }
}
