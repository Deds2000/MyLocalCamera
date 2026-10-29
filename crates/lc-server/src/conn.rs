//! Laço de aceitação e laço por conexão: handshake (`AwaitHello` →
//! `WELCOME`) e laço de sessão (`Active`).

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lc_protocol::json::{Bye, PcInfo, Reason, Welcome};
use lc_protocol::{type_name, Frame, FrameReader, Message};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, watch, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;

use crate::events::{CloseReason, ServerEvent};
use crate::handshake::{self, now_secs, write_msg, Accepted, Outcome, ServerCtx};
use crate::state::{transition, ConnState};
use crate::version::SERVER_PROTOCOL;

/// Tamanho do buffer de leitura por conexão.
const READ_BUF_SIZE: usize = 64 * 1024;

/// Parte servidor (compartilhada) dos dados por conexão.
#[derive(Clone)]
pub(crate) struct ConnShared {
    ctx: Arc<ServerCtx>,
    events_tx: mpsc::Sender<ServerEvent>,
    shutdown_rx: watch::Receiver<bool>,
    hello_timeout: Duration,
}

impl ConnShared {
    /// Monta os dados compartilhados.
    #[must_use]
    pub(crate) fn new(
        ctx: Arc<ServerCtx>,
        events_tx: mpsc::Sender<ServerEvent>,
        shutdown_rx: watch::Receiver<bool>,
        hello_timeout: Duration,
    ) -> Self {
        Self {
            ctx,
            events_tx,
            shutdown_rx,
            hello_timeout,
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
                let handle = tokio::spawn(run_conn(stream, peer, conn_id, permit, shared.clone()));
                conns.lock().unwrap_or_else(|p| p.into_inner()).push(handle);
            }
        }
    }
}

/// Laço por conexão:
/// 1. 1º quadro (`AwaitHello`, com prazo): `HELLO` → handshake; qualquer
///    outra coisa → `ProtocolError` (sem resposta);
/// 2. `WELCOME` enviado — aceito: libera o permit e entra na sessão;
///    negado: fecha **após** enviar;
/// 3. laço de sessão (`Active`): `PING`→`PONG`, `BYE`→encerra.
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
    } = shared;
    if let Err(e) = stream.set_nodelay(true) {
        tracing::warn!(conn_id, %peer, error = %e, "TCP_NODELAY falhou");
        emit_closed(&mut events_tx, conn_id, CloseReason::ConnectionLost).await;
        return;
    }

    let mut reader = FrameReader::new();
    let mut buf = [0u8; READ_BUF_SIZE];

    // ── 1) Primeiro quadro (AwaitHello) ──
    let first =
        read_first_frame(&mut stream, &mut reader, &mut buf, &mut shutdown_rx, hello_timeout).await;
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

    let mut state = ConnState::AwaitHello;
    if matches!(msg, Message::Hello(_)) {
        step(state, ConnState::Authenticating);
        state = ConnState::Authenticating;
    }

    // ── 2) Handshake (versão, auth pair/device + aprovação, sessão única) ──
    let outcome =
        handshake::perform(&ctx, &events_tx, &mut stream, &mut buf, conn_id, &msg, peer).await;
    match outcome {
        Outcome::Denied { reason, min_supported } => {
            // fecha **após** enviar o WELCOME (mesmo se o envio falhar)
            let _ = write_msg(&mut stream, &Message::Welcome(denied_welcome(reason, min_supported))).await;
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
            let welcome = Message::Welcome(accepted_welcome(&ctx, &session_id, audio_ticket, device_key));
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
            let reason = session_loop(&mut stream, &mut reader, &mut buf, conn_id, kick_rx, &mut shutdown_rx).await;
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

/// Laço de sessão (`Active`): `PING`→`PONG` (mesmo `u64`), `BYE`→encerra,
/// `HELLO` de novo→`ProtocolError`; o resto é logado e ignorado
/// (F1-T10/Fase 3). `kick_rx` disparado = substituição pelo mesmo
/// dispositivo: envia `BYE{reason:"replaced"}` e fecha com `Replaced`.
async fn session_loop(
    stream: &mut TcpStream,
    reader: &mut FrameReader,
    buf: &mut [u8],
    conn_id: u64,
    mut kick_rx: oneshot::Receiver<()>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> CloseReason {
    loop {
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
                            Message::Bye(_) => return CloseReason::PeerClosed,
                            Message::Hello(_) => return CloseReason::ProtocolError,
                            _ => tracing::debug!(
                                conn_id,
                                frame_type = type_name(frame.msg_type),
                                "mensagem ignorada na sessão (F1-T10/Fase 3)"
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
fn denied_welcome(reason: Reason, min_supported: Option<lc_protocol::json::ProtocolVersion>) -> Welcome {
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
    let _ = events_tx.send(ServerEvent::ConnectionClosed { conn_id, reason }).await;
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
