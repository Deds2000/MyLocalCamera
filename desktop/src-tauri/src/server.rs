use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64};

use lc_server::config::{
    DEFAULT_APPROVAL_TIMEOUT, DEFAULT_AUDIO_BLOCK_MAX, DEFAULT_AUTH_BLOCK, DEFAULT_AUTH_FAIL_MAX,
    DEFAULT_AUTH_FAIL_WINDOW, DEFAULT_CONN_RATE_MAX, DEFAULT_CONN_RATE_WINDOW,
    DEFAULT_HELLO_TIMEOUT, DEFAULT_LIVENESS_TIMEOUT, DEFAULT_MAX_UNAUTH_CONNS,
    DEFAULT_PING_INTERVAL, DEFAULT_STATS_INTERVAL, ServerConfig,
};
use lc_server::{ServerError, ServerHandle, start};

pub(crate) const REQUESTED_PORT: u16 = 48_650;

pub(crate) struct AppState {
    pub(crate) server: Mutex<Option<ServerHandle>>,
    pub(crate) pending_request_id: AtomicU64,
    pub(crate) shutting_down: AtomicBool,
    pub(crate) requested_port: u16,
    pub(crate) pc_name: String,
}

impl AppState {
    pub(crate) async fn start(data_dir: PathBuf) -> Result<Self, ServerError> {
        let pc_name = pc_name();
        let config = ServerConfig {
            bind_ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port: REQUESTED_PORT,
            data_dir,
            pc_name: pc_name.clone(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            max_unauth_conns: DEFAULT_MAX_UNAUTH_CONNS,
            hello_timeout: DEFAULT_HELLO_TIMEOUT,
            auto_approve: false,
            approval_timeout: DEFAULT_APPROVAL_TIMEOUT,
            auth_fail_max: DEFAULT_AUTH_FAIL_MAX,
            auth_fail_window: DEFAULT_AUTH_FAIL_WINDOW,
            auth_block: DEFAULT_AUTH_BLOCK,
            conn_rate_max: DEFAULT_CONN_RATE_MAX,
            conn_rate_window: DEFAULT_CONN_RATE_WINDOW,
            audio_block_max: DEFAULT_AUDIO_BLOCK_MAX,
            ping_interval: DEFAULT_PING_INTERVAL,
            liveness_timeout: DEFAULT_LIVENESS_TIMEOUT,
            stats_interval: DEFAULT_STATS_INTERVAL,
        };
        let server = start(config).await?;

        Ok(Self::from_server(server, REQUESTED_PORT, pc_name))
    }

    pub(crate) fn from_server(server: ServerHandle, requested_port: u16, pc_name: String) -> Self {
        Self {
            server: Mutex::new(Some(server)),
            pending_request_id: AtomicU64::new(0),
            shutting_down: AtomicBool::new(false),
            requested_port,
            pc_name,
        }
    }
}

pub(crate) fn lock_server(state: &AppState) -> std::sync::MutexGuard<'_, Option<ServerHandle>> {
    state
        .server
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

pub(crate) fn take_server(state: &AppState) -> Option<ServerHandle> {
    lock_server(state).take()
}

fn pc_name() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "LocalCamera PC".to_owned())
}
