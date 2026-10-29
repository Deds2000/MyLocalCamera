//! Servidor de rede do LocalCamera (desktop): listener TCP com fallback de
//! porta, limite de conexões não autenticadas e laço de leitura por conexão.
//! Sem dependência de Tauri; será usado por `desktop/src-tauri`.

pub mod config;
pub mod conn;
pub mod events;
pub mod state;
pub mod trust;

use std::io;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch, Semaphore};
use tokio::task::JoinHandle;

use crate::config::ServerConfig;
use crate::events::ServerEvent;

/// Capacidade da fila de eventos.
const EVENTS_CAPACITY: usize = 64;

/// Erros de inicialização do servidor.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// Falha ao ligar o listener (mesmo após o fallback de porta).
    #[error("falha ao ligar o listener: {0}")]
    Bind(#[from] io::Error),
}

/// Alça do servidor em execução.
pub struct ServerHandle {
    shutdown_tx: watch::Sender<bool>,
    accept: Option<JoinHandle<()>>,
    conns: Arc<Mutex<Vec<JoinHandle<()>>>>,
    events_rx: mpsc::Receiver<ServerEvent>,
    port: u16,
}

impl ServerHandle {
    /// Porta TCP efetiva (pode diferir da pedida quando houve fallback).
    #[must_use]
    pub fn local_port(&self) -> u16 {
        self.port
    }

    /// Recebedor dos eventos do servidor, na ordem de emissão.
    pub fn events(&mut self) -> &mut mpsc::Receiver<ServerEvent> {
        &mut self.events_rx
    }

    /// Sinaliza o shutdown, para de aceitar e espera todas as conexões
    /// fecharem (cada uma com motivo [`crate::events::CloseReason::Shutdown`]).
    /// Repetir a chamada é inócuo.
    pub async fn shutdown(&mut self) {
        let _ = self.shutdown_tx.send(true);
        if let Some(handle) = self.accept.take() {
            if let Err(e) = handle.await {
                tracing::error!(error = %e, "task do accept loop panicou");
            }
        }
        // solta o lock antes de awaitar as tasks
        let conns = {
            let mut guard = self.conns.lock().unwrap_or_else(|p| p.into_inner());
            std::mem::take(&mut *guard)
        };
        for conn in conns {
            if let Err(e) = conn.await {
                tracing::error!(error = %e, "task de conexão panicou");
            }
        }
    }
}

/// Inicia o servidor conforme a configuração.
///
/// Liga em `(bind_ip, port)`; se a porta estiver ocupada e `port != 0`,
/// escolhe uma porta livre (`0`), emite [`ServerEvent::PortChanged`] e
/// emite [`ServerEvent::Listening`] com a porta efetiva.
pub async fn start(cfg: ServerConfig) -> Result<ServerHandle, ServerError> {
    let (events_tx, events_rx) = mpsc::channel(EVENTS_CAPACITY);
    let listener = bind_listener(&cfg, &events_tx).await?;
    let port = listener.local_addr()?.port();
    let _ = events_tx.send(ServerEvent::Listening { port }).await;

    let unauth = Arc::new(Semaphore::new(cfg.max_unauth_conns));
    let next_id = Arc::new(AtomicU64::new(1));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let conns: Arc<Mutex<Vec<JoinHandle<()>>>> = Arc::new(Mutex::new(Vec::new()));
    let hello_timeout: Duration = cfg.hello_timeout;

    let accept = tokio::spawn(conn::accept_loop(
        listener,
        unauth,
        next_id,
        events_tx,
        conns.clone(),
        shutdown_rx,
        hello_timeout,
    ));

    Ok(ServerHandle {
        shutdown_tx,
        accept: Some(accept),
        conns,
        events_rx,
        port,
    })
}

/// Liga na porta pedida; se estiver ocupada (e a porta não for `0`), usa uma
/// porta livre e emite [`ServerEvent::PortChanged`].
async fn bind_listener(
    cfg: &ServerConfig,
    events_tx: &mpsc::Sender<ServerEvent>,
) -> Result<TcpListener, ServerError> {
    match TcpListener::bind((cfg.bind_ip, cfg.port)).await {
        Ok(listener) => Ok(listener),
        Err(e) if e.kind() == io::ErrorKind::AddrInUse && cfg.port != 0 => {
            let listener = TcpListener::bind((cfg.bind_ip, 0)).await?;
            let actual = listener.local_addr()?.port();
            let _ = events_tx
                .send(ServerEvent::PortChanged {
                    requested: cfg.port,
                    actual,
                })
                .await;
            Ok(listener)
        }
        Err(e) => Err(ServerError::Bind(e)),
    }
}
