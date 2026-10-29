//! Laço de aceitação e laço por conexão: handshake (`AwaitHello` →
//! `WELCOME`) e laço de sessão (`Active`).

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lc_protocol::json::{AudioTicket, Bye, PcInfo, Reason, Welcome};
use lc_protocol::{flags, type_name, Frame, FrameReader, Message};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, watch, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;
use tracing::Instrument;

use crate::audio::conn::{run_audio, AudioConn};
use crate::events::{CloseReason, ServerEvent};
use crate::handshake::{self, now_secs, write_msg, Accepted, Outcome, ServerCtx};
use crate::keepalive::{mono_ms, Keepalive};
use crate::redact::short;
use crate::sessions::SessionCounters;
use crate::state::{transition, ConnState};
use crate::version::SERVER_PROTOCOL;

/// Tamanho do buffer de leitura por conexão.
const READ_BUF_SIZE: usize = 64 * 1024;

/// Timings de keepalive e estatísticas (F1-T10), agrupados para manter a
/// assinatura de [`ConnShared::new`] enxuta.
#[derive(Debug, Clone, Copy)]
pub(crate) struct KeepaliveTiming {
    /// Intervalo dos `PING` de keepalive.
    pub ping_interval: Duration,
    /// Sem `PONG` por este intervalo → `ConnectionLost`.
    pub liveness_timeout: Duration,
    /// Intervalo do agregador de estatísticas.
    pub stats_interval: Duration,
}

/// Parte servidor (compartilhada) dos dados por conexão.
#[derive(Clone)]
pub(crate) struct ConnShared {
    ctx: Arc<ServerCtx>,
    events_tx: mpsc::Sender<ServerEvent>,
    shutdown_rx: watch::Receiver<bool>,
    hello_timeout: Duration,
    /// Máximo de um bloco `AUDIO` no socket de áudio (F1-T09b).
    audio_block_max: usize,
    /// Keepalive e estatísticas (F1-T10).
    timing: KeepaliveTiming,
}

impl ConnShared {
    /// Monta os dados compartilhados.
    #[must_use]
    pub(crate) fn new(
        ctx: Arc<ServerCtx>,
        events_tx: mpsc::Sender<ServerEvent>,
        shutdown_rx: watch::Receiver<bool>,
        hello_timeout: Duration,
        audio_block_max: usize,
        timing: KeepaliveTiming,
    ) -> Self {
        Self {
            ctx,
            events_tx,
            shutdown_rx,
            hello_timeout,
            audio_block_max,
            timing,
        }
    }
}

/// Aceita conexões até o shutdown.
///
/// Cada conexão aceita recebe um `conn_id` crescente, `TCP_NODELAY` e um
/// permit do limite de não autenticadas; se o limite estoura, o socket é
/// fechado na hora (sem criar task) e é emitido `TooManyUnauth`.
pub(super) async fn accept_loop(
    listener: TcpListener,
    shared: ConnShared,
    unauth: Arc<Semaphore>,
    next_id: Arc<AtomicU64>,
    conns: Arc<Mutex<Vec<JoinHandle<()>>>>,
) {
    let ConnShared {
        ctx,
        events_tx,
        mut shutdown_rx,
        ..
    } = shared.clone();
    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => break,
            accepted = listener.accept() => {
                let (stream, peer) = match accepted {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!(error = %e, "accept falhou; segue aceitando");
                        continue;
                    }
                };
                let conn_id = next_id.fetch_add(1, Ordering::Relaxed);
                // F1-T15 — antes de qualquer leitura: taxa de conexões
                // do IP e bloqueio por falhas de auth. Recusado: o
                // socket fecha no drop, **sem** resposta.
                let ip = peer.ip();
                let now = now_secs();
                let rate_ok = ctx
                    .limiters
                    .conn_rate
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .allow(ip, now);
                if !rate_ok
                    || ctx
                        .limiters
                        .auth
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .is_blocked(ip, now)
                {
                    tracing::debug!(conn_id, %peer, "conexão recusada por limitador");
                    let _ = events_tx
                        .send(ServerEvent::ConnectionClosed {
                            conn_id,
                            reason: CloseReason::RateLimited,
                        })
                        .await;
                    continue;
                }
                let permit = match unauth.clone().try_acquire_owned() {
                    Ok(p) => p,
                    Err(_) => {
                        // o drop do `stream` (fim do braço) fecha o socket
                        let _ = events_tx
                            .send(ServerEvent::ConnectionClosed {
                                conn_id,
                                reason: CloseReason::TooManyUnauth,
                            })
                            .await;
                        continue;
                    }
                };
                let _ = events_tx
                    .send(ServerEvent::ConnectionOpened { conn_id, peer })
                    .await;
                let handle = tokio::spawn(
                    run_conn(stream, peer, conn_id, permit, shared.clone())
                        .instrument(tracing::info_span!("conn", conn = conn_id)),
                );
                conns.lock().unwrap_or_else(|p| p.into_inner()).push(handle);
            }
        }
    }
}

/// Laço por conexão:
/// 1. 1º quadro (`AwaitHello`, com prazo): `HELLO` → handshake;
///    `AUDIO_HELLO` → socket de áudio (F1-T09b); qualquer outra coisa
///    → `ProtocolError` (sem resposta);
/// 2. `WELCOME` enviado — aceito: libera o permit e entra na sessão;
///    negado: fecha **após** enviar;
/// 3. laço de sessão (`Active`): `PING`→`PONG`, `AUDIO_TICKET_REQUEST`
///    →`AUDIO_TICKET`, `BYE`→encerra.
async fn run_conn(
    mut stream: TcpStream,
    peer: SocketAddr,
    conn_id: u64,
    permit: OwnedSemaphorePermit,
    shared: ConnShared,
) {
    let ConnShared {
        ctx,
        mut events_tx,
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
        emit_closed(&mut events_tx, conn_id, CloseReason::ConnectionLost).await;
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
            emit_closed(&mut events_tx, conn_id, reason).await;
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
                    emit_closed(&mut events_tx, conn_id, CloseReason::ProtocolError).await;
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
        emit_closed(&mut events_tx, conn_id, reason).await;
        return;
    }

    let mut state = ConnState::AwaitHello;
    if matches!(msg, Message::Hello(_)) {
        step(state, ConnState::Authenticating);
        state = ConnState::Authenticating;
    }

    // ── 2) Handshake (versão, auth pair/device + aprovação, sessão única) ──
    let outcome =
        handshake::perform(&ctx, &events_tx, &mut stream, &mut buf, conn_id, &msg, peer).await;
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
            emit_closed(&mut events_tx, conn_id, CloseReason::AuthDenied).await;
        }
        Outcome::Close(reason) => {
            // fora de ordem: fechar calado, sem resposta
            step(state, ConnState::Closing);
            emit_closed(&mut events_tx, conn_id, reason).await;
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
                emit_closed(&mut events_tx, conn_id, CloseReason::ConnectionLost).await;
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
            let session_data = SessionData {
                ctx: ctx.clone(),
                session_id: session_id.clone(),
                counters,
                ping_interval,
                liveness_timeout,
            };
            let reason = session_loop(
                &mut stream,
                &mut reader,
                &mut buf,
                &session_data,
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
            emit_closed(&mut events_tx, conn_id, reason).await;
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

/// Dados da sessão para o laço ativo (`Active`) — agrupa o que é
/// específico da sessão (evita `too_many_arguments`).
struct SessionData {
    /// Contexto do servidor (store de tickets p/ `AUDIO_TICKET_REQUEST`).
    ctx: Arc<ServerCtx>,
    /// `sessionId` da sessão.
    session_id: String,
    /// Contadores da sessão (vídeo + RTT; `None` na race de
    /// substituição — a sessão encerra via `kick_rx`).
    counters: Option<Arc<SessionCounters>>,
    /// Intervalo dos `PING` de keepalive (F1-T10).
    ping_interval: Duration,
    /// Sem `PONG` por este intervalo → `ConnectionLost` (F1-T10).
    liveness_timeout: Duration,
}

/// Laço de sessão (`Active`): `PING`→`PONG` (mesmo `u64`),
/// `AUDIO_TICKET_REQUEST`→`AUDIO_TICKET` (F1-T09b), `BYE`→encerra,
/// `HELLO` de novo→`ProtocolError`; o resto é logado e ignorado
/// (Fase 3). Keepalive (F1-T10): envia `PING` a cada `ping_interval`,
/// mede o RTT nos `PONG` e fecha com `ConnectionLost` sem `PONG` por
/// `liveness_timeout` (nenhum outro tráfego substitui o `PONG`).
/// `kick_rx` disparado = substituição pelo mesmo dispositivo: envia
/// `BYE{reason:"replaced"}` e fecha com `Replaced`.
async fn session_loop(
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
fn accepted_welcome(
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
fn denied_welcome(
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
async fn emit_closed(events_tx: &mut mpsc::Sender<ServerEvent>, conn_id: u64, reason: CloseReason) {
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
fn purge_ticket(ctx: &ServerCtx, session_id: &str) {
    ctx.tickets
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .purge(Some(session_id), now_secs());
}

/// Valida uma transição da máquina de estados (auxílio de programação);
/// transição inválida é logada, mas não derruba a conexão.
fn step(from: ConnState, to: ConnState) {
    if let Err(e) = transition(from, to) {
        tracing::error!(from = ?from, to = ?to, "transição de estado inesperada: {e}");
    }
}
