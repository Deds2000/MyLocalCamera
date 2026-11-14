use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::sync::{watch, Semaphore};
use tokio::task::JoinHandle;
use tracing::Instrument;

use super::connection::run_conn;
use crate::events::{CloseReason, EventSink, ServerEvent};
use crate::handshake::{now_secs, ServerCtx};
/// Frequência para recolher tarefas de conexão concluídas mesmo sem novas
/// conexões ou chamadas a `active_connections()`.
const CONNECTION_REAP_INTERVAL: Duration = Duration::from_millis(100);

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
    pub(super) ctx: Arc<ServerCtx>,
    pub(super) events_tx: EventSink,
    pub(super) shutdown_rx: watch::Receiver<bool>,
    pub(super) hello_timeout: Duration,
    /// Máximo de um bloco `AUDIO` no socket de áudio (F1-T09b).
    pub(super) audio_block_max: usize,
    /// Keepalive e estatísticas (F1-T10).
    pub(super) timing: KeepaliveTiming,
}

impl ConnShared {
    /// Monta os dados compartilhados.
    #[must_use]
    pub(crate) fn new(
        ctx: Arc<ServerCtx>,
        events_tx: EventSink,
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
pub(crate) async fn accept_loop(
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
    let mut reap = tokio::time::interval(CONNECTION_REAP_INTERVAL);
    reap.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => break,
            _ = reap.tick() => reap_finished(&conns).await,
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

/// Remove e aguarda tarefas terminadas fora do lock. Awaitar o handle é o
/// que permite observar/logar panics em vez de descartá-los silenciosamente.
async fn reap_finished(conns: &Arc<Mutex<Vec<JoinHandle<()>>>>) {
    let finished = {
        let mut guard = conns.lock().unwrap_or_else(|p| p.into_inner());
        let mut finished = Vec::new();
        let mut i = 0;
        while i < guard.len() {
            if guard[i].is_finished() {
                finished.push(guard.swap_remove(i));
            } else {
                i += 1;
            }
        }
        finished
    };
    for handle in finished {
        if let Err(error) = handle.await {
            tracing::error!(error = %error, "task de conexão panicou");
        }
    }
}
