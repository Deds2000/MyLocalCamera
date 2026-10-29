//! Configuração do servidor de rede.

use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time::Duration;

/// Porta padrão do protocolo LocalCamera (ver `docs/CHEATSHEET.md`).
pub const DEFAULT_PORT: u16 = 48650;
/// Máximo de conexões não autenticadas simultâneas.
pub const DEFAULT_MAX_UNAUTH_CONNS: usize = 8;
/// Prazo para o primeiro quadro (`HELLO`) chegar.
pub const DEFAULT_HELLO_TIMEOUT: Duration = Duration::from_secs(5);
/// Prazo da aprovação no PC (30 s; ver `docs/CHEATSHEET.md`).
pub const DEFAULT_APPROVAL_TIMEOUT: Duration = Duration::from_secs(30);
/// Falhas de autenticação do mesmo IP que disparam o bloqueio (F1-T15).
pub const DEFAULT_AUTH_FAIL_MAX: u32 = 5;
/// Janela de contagem das falhas de autenticação, em segundos (F1-T15).
pub const DEFAULT_AUTH_FAIL_WINDOW: u64 = 60;
/// Duração do bloqueio do IP, em segundos (F1-T15).
pub const DEFAULT_AUTH_BLOCK: u64 = 300;
/// Novas conexões do mesmo IP permitidas na janela (F1-T15).
pub const DEFAULT_CONN_RATE_MAX: usize = 20;
/// Janela de contagem das novas conexões, em segundos (F1-T15).
pub const DEFAULT_CONN_RATE_WINDOW: u64 = 10;

/// Configuração do servidor de rede.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// IP de escuta (`0.0.0.0` em produção, `127.0.0.1` em testes).
    pub bind_ip: IpAddr,
    /// Porta a escutar (`0` = escolher uma livre).
    pub port: u16,
    /// Diretório de dados do app (chave, confiança etc.).
    pub data_dir: PathBuf,
    /// Nome do PC (usado no `WELCOME`).
    pub pc_name: String,
    /// Versão do app (usada no `WELCOME`).
    pub app_version: String,
    /// Máximo de conexões não autenticadas simultâneas.
    pub max_unauth_conns: usize,
    /// Prazo para o primeiro quadro (`HELLO`) chegar.
    pub hello_timeout: Duration,
    /// Aprova pareamentos automaticamente **em build de debug** (em
    /// release é ignorado com aviso no log). Padrão **sempre** `false`.
    pub auto_approve: bool,
    /// Prazo da aprovação no PC (`PENDING` + timeout do fluxo de
    /// aprovação — F1-T06).
    pub approval_timeout: Duration,
    /// Falhas de autenticação do mesmo IP em `auth_fail_window` que
    /// disparam o bloqueio (F1-T15).
    pub auth_fail_max: u32,
    /// Janela de contagem das falhas de autenticação, em segundos (F1-T15).
    pub auth_fail_window: u64,
    /// Duração do bloqueio do IP, em segundos (F1-T15).
    pub auth_block: u64,
    /// Novas conexões do mesmo IP em `conn_rate_window` (F1-T15);
    /// acima disso, as seguintes são fechadas até a janela esvaziar.
    pub conn_rate_max: usize,
    /// Janela de contagem das novas conexões, em segundos (F1-T15).
    pub conn_rate_window: u64,
}

impl ServerConfig {
    /// Configuração para testes: `127.0.0.1`, porta `0` e hello de 300 ms.
    #[must_use]
    pub fn for_tests(dir: PathBuf, auto_approve: bool) -> Self {
        Self {
            bind_ip: IpAddr::from(Ipv4Addr::LOCALHOST),
            port: 0,
            data_dir: dir,
            pc_name: "pc-teste".to_owned(),
            app_version: "0.0.0-test".to_owned(),
            max_unauth_conns: DEFAULT_MAX_UNAUTH_CONNS,
            hello_timeout: Duration::from_millis(300),
            auto_approve,
            approval_timeout: DEFAULT_APPROVAL_TIMEOUT,
            auth_fail_max: DEFAULT_AUTH_FAIL_MAX,
            auth_fail_window: DEFAULT_AUTH_FAIL_WINDOW,
            auth_block: DEFAULT_AUTH_BLOCK,
            // Solto: testes existentes abrem > 20 conexões do mesmo IP
            // (ex. `unauth_limit_enforced`); F1-T15 usa valores
            // explícitos nos próprios testes.
            conn_rate_max: 1_000,
            conn_rate_window: DEFAULT_CONN_RATE_WINDOW,
        }
    }
}
