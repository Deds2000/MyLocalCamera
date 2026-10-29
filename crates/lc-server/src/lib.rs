//! Servidor de rede do LocalCamera (desktop): listener TCP com fallback de
//! porta, limite de conexões não autenticadas, handshake (`HELLO` →
//! `WELCOME`) e laço de sessão por conexão.
//! Sem dependência de Tauri; será usado por `desktop/src-tauri`.

pub mod audio;
pub mod config;
pub mod conn;
pub mod events;
pub mod handshake;
pub mod keepalive;
pub mod sessions;
pub mod state;
pub mod stats;
pub mod trust;
pub mod version;

use std::io;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch, Semaphore};
use tokio::task::JoinHandle;

use crate::audio::TicketStore;
use crate::config::ServerConfig;
use crate::events::{Decision, ServerEvent};
use crate::handshake::ServerCtx;
use crate::sessions::SessionRegistry;
use crate::trust::ratelimit::{AuthLimiter, ConnRateLimiter, Limiters};
use crate::trust::{ApprovalManager, TrustError, TrustStore, UsbWindow};

/// Capacidade da fila de eventos.
const EVENTS_CAPACITY: usize = 64;

/// Erros de inicialização do servidor.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// Falha ao ligar o listener (mesmo após o fallback de porta).
    #[error("falha ao ligar o listener: {0}")]
    Bind(#[from] io::Error),
    /// Falha ao abrir a loja de confiança (`trust.json`).
    #[error("loja de confiança: {0}")]
    Trust(#[from] TrustError),
}

/// Alça do servidor em execução.
pub struct ServerHandle {
    shutdown_tx: watch::Sender<bool>,
    accept: Option<JoinHandle<()>>,
    conns: Arc<Mutex<Vec<JoinHandle<()>>>>,
    events_tx: mpsc::Sender<ServerEvent>,
    events_rx: mpsc::Receiver<ServerEvent>,
    port: u16,
    ctx: Arc<ServerCtx>,
}

impl ServerHandle {
    /// Porta TCP efetiva (pode diferir da pedida quando houve fallback).
    #[must_use]
    pub fn local_port(&self) -> u16 {
        self.port
    }

    /// Loja de confiança (token de pareamento e dispositivos confiáveis) —
    /// usada pela UI no fluxo de aprovação (F1-T06) e em testes.
    #[must_use]
    pub fn trust(&self) -> Arc<Mutex<TrustStore>> {
        self.ctx.trust.clone()
    }

    /// Registro da sessão ativa (F1-T09b/F1-T10): contadores
    /// (`SessionRegistry::counters`) e canal de áudio. Usado pela UI e
    /// pelos testes.
    #[must_use]
    pub fn sessions(&self) -> Arc<SessionRegistry> {
        self.ctx.sessions.clone()
    }

    /// Decide um pedido de aprovação pendente (`ApprovalRequested`).
    ///
    /// Devolve `false` se o `request_id` é desconhecido ou já foi
    /// resolvido (decisão ignorada; o estado não muda).
    pub fn decide(&self, request_id: u64, decision: Decision) -> bool {
        self.ctx.approvals.decide(request_id, decision)
    }

    /// Abre a janela de aprovação USB por 15 s (F1-T07): o primeiro
    /// `HELLO` válido de `127.0.0.1`/`::1` é aprovado sem diálogo
    /// (`ApproveAlways`; o celular recebe `deviceKey`). O token de
    /// pareamento continua obrigatório; peers remotos não consomem a
    /// janela. Emite [`ServerEvent::UsbWindowOpened`] (e
    /// [`ServerEvent::UsbWindowClosed`] no fim da janela).
    pub async fn open_usb_window(&mut self) {
        let now = handshake::now_secs();
        self.ctx
            .usb_window
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .open(now);
        let _ = self.events_tx.send(ServerEvent::UsbWindowOpened).await;
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

    let trust = Arc::new(Mutex::new(TrustStore::open(&cfg.data_dir)?));
    let pc_id = trust
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .pc_id()
        .to_owned();
    let limiters = Arc::new(Limiters {
        auth: Mutex::new(AuthLimiter::new(
            cfg.auth_fail_max,
            cfg.auth_fail_window,
            cfg.auth_block,
        )),
        conn_rate: Mutex::new(ConnRateLimiter::new(
            cfg.conn_rate_max,
            cfg.conn_rate_window,
        )),
    });
    let ctx = Arc::new(ServerCtx {
        trust,
        tickets: Arc::new(Mutex::new(TicketStore::new())),
        sessions: Arc::new(SessionRegistry::new()),
        approvals: Arc::new(ApprovalManager::new()),
        usb_window: Arc::new(Mutex::new(UsbWindow::new())),
        limiters,
        approval_timeout: cfg.approval_timeout,
        auto_approve: cfg.auto_approve,
        pc_id,
        pc_name: cfg.pc_name.clone(),
        app_version: cfg.app_version.clone(),
    });

    let unauth = Arc::new(Semaphore::new(cfg.max_unauth_conns));
    let next_id = Arc::new(AtomicU64::new(1));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let conns: Arc<Mutex<Vec<JoinHandle<()>>>> = Arc::new(Mutex::new(Vec::new()));
    let hello_timeout: Duration = cfg.hello_timeout;

    let shared = conn::ConnShared::new(
        ctx.clone(),
        events_tx.clone(),
        shutdown_rx,
        hello_timeout,
        cfg.audio_block_max,
        conn::KeepaliveTiming {
            ping_interval: cfg.ping_interval,
            liveness_timeout: cfg.liveness_timeout,
            stats_interval: cfg.stats_interval,
        },
    );
    let accept = tokio::spawn(conn::accept_loop(
        listener,
        shared,
        unauth,
        next_id,
        conns.clone(),
    ));

    Ok(ServerHandle {
        shutdown_tx,
        accept: Some(accept),
        conns,
        events_tx,
        events_rx,
        port,
        ctx,
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
