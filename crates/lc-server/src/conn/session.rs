use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use lc_protocol::json::{AudioTicket, Bye, PcInfo, Reason, Welcome};
use lc_protocol::{flags, type_name, FrameReader, Message};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::{oneshot, watch};

use crate::events::{CloseReason, EventSink, ServerEvent};
use crate::handshake::{now_secs, write_msg, ServerCtx};
use crate::keepalive::{mono_ms, Keepalive};
use crate::sessions::SessionCounters;
use crate::state::{transition, ConnState};
use crate::version::SERVER_PROTOCOL;
/// Dados da sessão para o laço ativo (`Active`) — agrupa o que é
/// específico da sessão (evita `too_many_arguments`).
pub(super) struct SessionData {
    /// Contexto do servidor (store de tickets p/ `AUDIO_TICKET_REQUEST`).
    pub(super) ctx: Arc<ServerCtx>,
    /// `sessionId` da sessão.
    pub(super) session_id: String,
    /// Contadores da sessão (vídeo + RTT; `None` na race de
    /// substituição — a sessão encerra via `kick_rx`).
    pub(super) counters: Option<Arc<SessionCounters>>,
    /// Intervalo dos `PING` de keepalive (F1-T10).
    pub(super) ping_interval: Duration,
    /// Sem `PONG` por este intervalo → `ConnectionLost` (F1-T10).
    pub(super) liveness_timeout: Duration,
}

/// Laço de sessão (`Active`): `PING`→`PONG` (mesmo `u64`),
/// `AUDIO_TICKET_REQUEST`→`AUDIO_TICKET` (F1-T09b), `BYE`→encerra,
/// `HELLO` de novo→`ProtocolError`; o resto é logado e ignorado
/// (Fase 3). Keepalive (F1-T10): envia `PING` a cada `ping_interval`,
/// mede o RTT nos `PONG` e fecha com `ConnectionLost` sem `PONG` por
/// `liveness_timeout` (nenhum outro tráfego substitui o `PONG`).
/// `kick_rx` disparado = substituição pelo mesmo dispositivo: envia
/// `BYE{reason:"replaced"}` e fecha com `Replaced`.
pub(super) async fn session_loop(
    stream: &mut TcpStream,
    reader: &mut FrameReader,
    buf: &mut [u8],
    session: &SessionData,
    conn_id: u64,
    mut kick_rx: oneshot::Receiver<()>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> CloseReason {
    let SessionData {
        ping_interval,
        liveness_timeout,
        ..
    } = session;
    let mut keepalive = Keepalive::new(*liveness_timeout);
    let mut ticker = tokio::time::interval(*ping_interval);
    // O 1º tick é imediato: descarta para o 1º `PING` sair após 1 ciclo.
    ticker.tick().await;
    loop {
        let liveness = tokio::time::sleep_until(keepalive.deadline().into());
        tokio::pin!(liveness);
        tokio::select! {
            res = stream.read(buf) => match res {
                Ok(0) => return CloseReason::PeerClosed,
                Ok(n) => match reader.push(&buf[..n]) {
                    Ok(frames) => for frame in frames {
                        let msg = match Message::from_frame(&frame) {
                            Ok(m) => m,
                            Err(e) => {
                                tracing::warn!(conn_id, error = %e, "frame inválido na sessão; fechando");
                                return CloseReason::ProtocolError;
                            }
                        };
                        match msg {
                            Message::Ping(v) => {
                                if let Err(e) = write_msg(stream, &Message::Pong(v)).await {
                                    tracing::warn!(conn_id, error = %e, "falha ao enviar PONG");
                                    return CloseReason::ConnectionLost;
                                }
                            }
                            Message::Pong(v) => {
                                // Keepalive (F1-T10): RTT (EWMA) + renova
                                // o liveness. Só `PONG` mantém vivo.
                                keepalive.on_pong(v, mono_ms());
                                if let (Some(c), Some(rtt)) = (&session.counters, keepalive.rtt_ms()) {
                                    c.rtt_ewma_ms
                                        .store(rtt_to_u32(rtt), Ordering::Relaxed);
                                }
                            }
                            Message::Video { stream, flags, data, .. } => {
                                count_video(session.counters.as_ref(), stream, flags, data.len());
                            }
                            Message::AudioTicketRequest => {
                                // Sessão ativa: emite um novo ticket de
                                // uso único (invalida o anterior não
                                // usado — F1-T09b).
                                let ticket = session
                                    .ctx
                                    .tickets
                                    .lock()
                                    .unwrap_or_else(|p| p.into_inner())
                                    .issue(&session.session_id, now_secs());
                                if let Err(e) = write_msg(stream, &Message::AudioTicket(AudioTicket { ticket })).await {
                                    tracing::warn!(conn_id, error = %e, "falha ao enviar AUDIO_TICKET");
                                    return CloseReason::ConnectionLost;
                                }
                            }
                            Message::Bye(_) => return CloseReason::PeerClosed,
                            Message::Hello(_) => return CloseReason::ProtocolError,
                            _ => tracing::debug!(
                                conn_id,
                                frame_type = type_name(frame.msg_type),
                                "mensagem ignorada na sessão (Fase 3)"
                            ),
                        }
                    },
                    Err(e) => {
                        tracing::warn!(conn_id, error = %e, "erro de protocolo na sessão");
                        return CloseReason::ProtocolError;
                    }
                },
                Err(e) => {
                    tracing::warn!(conn_id, error = %e, "erro de leitura");
                    return CloseReason::ConnectionLost;
                }
            },
            // Keepalive (F1-T10): a cada `ping_interval` envia `PING`
            // (relógio monotônico em ms); se o liveness já estourou
            // (sem `PONG`), fecha com `ConnectionLost`.
            _ = ticker.tick() => {
                if keepalive.is_dead(std::time::Instant::now()) {
                    tracing::info!(conn_id, "sem PONG por {} ms; conexão perdida", liveness_timeout.as_millis());
                    return CloseReason::ConnectionLost;
                }
                if let Err(e) = write_msg(stream, &Message::Ping(mono_ms())).await {
                    tracing::warn!(conn_id, error = %e, "falha ao enviar PING");
                    return CloseReason::ConnectionLost;
                }
            }
            // Keepalive (F1-T10): prazo exato do liveness (atualizado a
            // cada `PONG` — o deadline é recalculado a cada iteração).
            _ = &mut liveness => {
                tracing::info!(conn_id, "liveness timeout ({} ms sem PONG); conexão perdida", liveness_timeout.as_millis());
                return CloseReason::ConnectionLost;
            }
            _ = &mut kick_rx => {
                let bye = Message::Bye(Bye { reason: "replaced".to_owned() });
                if let Err(e) = write_msg(stream, &bye).await {
                    tracing::warn!(conn_id, error = %e, "falha ao enviar BYE(replaced)");
                }
                return CloseReason::Replaced;
            }
            _ = shutdown_rx.changed() => return CloseReason::Shutdown,
        }
    }
}

/// `WELCOME` aceito (sessão ativa).
pub(super) fn accepted_welcome(
    ctx: &ServerCtx,
    session_id: &str,
    audio_ticket: String,
    device_key: Option<String>,
) -> Welcome {
    Welcome {
        accepted: true,
        protocol: Some(SERVER_PROTOCOL),
        session_id: Some(session_id.to_owned()),
        audio_ticket: Some(audio_ticket),
        device_key,
        pc: Some(PcInfo {
            id: ctx.pc_id.clone(),
            name: ctx.pc_name.clone(),
            app_version: ctx.app_version.clone(),
        }),
        reason: None,
        min_supported: None,
    }
}

/// `WELCOME` recusado (motivo + `minSupported` quando for versão).
pub(super) fn denied_welcome(
    reason: Reason,
    min_supported: Option<lc_protocol::json::ProtocolVersion>,
) -> Welcome {
    Welcome {
        accepted: false,
        protocol: None,
        session_id: None,
        audio_ticket: None,
        device_key: None,
        pc: None,
        reason: Some(reason),
        min_supported,
    }
}

/// Emite `ConnectionClosed`; o socket fecha com o drop do stream.
pub(super) async fn emit_closed(events_tx: &EventSink, conn_id: u64, reason: CloseReason) {
    let _ = events_tx
        .send(ServerEvent::ConnectionClosed { conn_id, reason })
        .await;
}

/// Conta um quadro `VIDEO` aceito (F1-T10): `video_frames[stream-1]`,
/// `video_bytes` por stream e `keyframes` (bit0 de `flags`).
/// `None` (race de substituição) → não conta; a sessão encerra via
/// `kick_rx`.
fn count_video(counters: Option<&Arc<SessionCounters>>, stream: u8, flags: u8, payload_len: usize) {
    let Some(c) = counters else { return };
    // Índices: 0 = stream 1, 1 = stream 2, 2 = reservado.
    let idx = match stream {
        1 => 0,
        2 => 1,
        _ => 2,
    };
    c.video_frames[idx].fetch_add(1, Ordering::Relaxed);
    c.video_bytes[idx].fetch_add(
        u64::try_from(payload_len).expect("payload ≤ 8 MiB cabe em u64"),
        Ordering::Relaxed,
    );
    if (flags & flags::KEYFRAME) != 0 {
        c.keyframes.fetch_add(1, Ordering::Relaxed);
    }
}

/// Converte o RTT (EWMA, f32 em ms) para o `AtomicU32` dos contadores;
/// RTT fora de [0, 65 535] ms (impossível na prática) satura.
fn rtt_to_u32(rtt: f32) -> u32 {
    // clamp garante [0, 65 535] → o cast não satura nem arredonda.
    rtt.clamp(0.0, 65_535.0).round() as u32
}

/// Descarta o ticket de áudio (ainda não usado) da sessão encerrada.
pub(super) fn purge_ticket(ctx: &ServerCtx, session_id: &str) {
    ctx.tickets
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .purge(Some(session_id), now_secs());
}

/// Valida uma transição da máquina de estados (auxílio de programação);
/// transição inválida é logada, mas não derruba a conexão.
pub(super) fn step(from: ConnState, to: ConnState) {
    if let Err(e) = transition(from, to) {
        tracing::error!(from = ?from, to = ?to, "transição de estado inesperada: {e}");
    }
}
