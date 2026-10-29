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
        }
    }
}
