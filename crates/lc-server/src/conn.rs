//! Laço de aceitação e laço de leitura por conexão.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lc_protocol::{type_name, FrameReader};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;

use crate::events::{CloseReason, ServerEvent};

/// Tamanho do buffer de leitura por conexão.
const READ_BUF_SIZE: usize = 64 * 1024;

/// Aceita conexões até o shutdown.
///
/// Cada conexão aceita recebe um `conn_id` crescente, `TCP_NODELAY` e um
/// permit do limite de não autenticadas; se o limite estoura, o socket é
/// fechado na hora (sem criar task) e é emitido `TooManyUnauth`.
pub(super) async fn accept_loop(
    listener: TcpListener,
    unauth: Arc<Semaphore>,
    next_id: Arc<AtomicU64>,
    events_tx: mpsc::Sender<ServerEvent>,
    conns: Arc<Mutex<Vec<JoinHandle<()>>>>,
    mut shutdown_rx: watch::Receiver<bool>,
    hello_timeout: Duration,
) {
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
                let handle = tokio::spawn(run_conn(
                    stream,
                    peer,
                    conn_id,
                    permit,
                    events_tx.clone(),
                    shutdown_rx.clone(),
                    hello_timeout,
                ));
                conns.lock().unwrap_or_else(|p| p.into_inner()).push(handle);
            }
        }
    }
}

/// Laço por conexão: lê bytes (cancel-safe), alimenta o [`FrameReader`] e
/// fecha com o motivo adequado. O `permit` é liberado automaticamente ao
/// fim da função (o F1-T04b passará a soltá-lo após autenticar).
async fn run_conn(
    mut stream: TcpStream,
    peer: SocketAddr,
    conn_id: u64,
    _permit: OwnedSemaphorePermit,
    events_tx: mpsc::Sender<ServerEvent>,
    mut shutdown_rx: watch::Receiver<bool>,
    hello_timeout: Duration,
) {
    if let Err(e) = stream.set_nodelay(true) {
        tracing::warn!(conn_id, %peer, error = %e, "TCP_NODELAY falhou");
        let _ = events_tx
            .send(ServerEvent::ConnectionClosed {
                conn_id,
                reason: CloseReason::ConnectionLost,
            })
            .await;
        return;
    }

    let mut reader = FrameReader::new();
    let mut buf = [0u8; READ_BUF_SIZE];
    let hello_deadline = tokio::time::sleep(hello_timeout);
    tokio::pin!(hello_deadline);
    let mut hello_seen = false;

    let reason = loop {
        tokio::select! {
            res = stream.read(&mut buf) => {
                match res {
                    Ok(0) => break CloseReason::PeerClosed,
                    Ok(n) => match reader.push(&buf[..n]) {
                        Ok(frames) => {
                            for frame in frames {
                                tracing::debug!(
                                    conn_id,
                                    %peer,
                                    frame_type = type_name(frame.msg_type),
                                    payload_len = frame.payload.len(),
                                    "frame recebido"
                                );
                                if !hello_seen {
                                    // neste cartão: só logar o 1º frame;
                                    // o handshake real é o F1-T04b.
                                    hello_seen = true;
                                    tracing::debug!(conn_id, %peer, "primeiro frame recebido");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(conn_id, %peer, error = %e, "erro de protocolo; fechando");
                            break CloseReason::ProtocolError;
                        }
                    },
                    Err(e) => {
                        tracing::warn!(conn_id, %peer, error = %e, "erro de leitura");
                        break CloseReason::ConnectionLost;
                    }
                }
            }
            _ = &mut hello_deadline, if !hello_seen => break CloseReason::HelloTimeout,
            _ = shutdown_rx.changed() => break CloseReason::Shutdown,
        }
    };

    // o drop do `stream` (fim da função) fecha o socket
    let _ = events_tx
        .send(ServerEvent::ConnectionClosed { conn_id, reason })
        .await;
}
