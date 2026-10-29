//! Limitadores de segurança (F1-T15): falhas de autenticação por IP e
//! taxa de novas conexões por IP.
//!
//! O relógio é **injetado** (`now` em segundos Unix; nunca
//! `Instant::now()` aqui) para os testes usarem um relógio virtual. Em
//! produção, `conn.rs`/`handshake.rs` passam `handshake::now_secs()`.
//!
//! - [`AuthLimiter`]: `max` falhas em `window_secs` do mesmo IP → bloqueio
//!   de `block_secs`; sucesso zera o contador.
//! - [`ConnRateLimiter`]: mais de `max` conexões em `window_secs` do mesmo
//!   IP → recusar as seguintes até a janela esvaziar (sem bloqueio longo).
//!
//! Os mapas limpam entradas antigas a cada consulta, para não crescerem
//! sem limite (IPs que nunca mais aparecem saem do mapa).

use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Mutex;

/// Estado por IP no [`AuthLimiter`].
#[derive(Debug)]
struct FailState {
    /// Momentos (s) das falhas da janela atual.
    failures: VecDeque<u64>,
    /// IP bloqueado até este momento (s); `None` = sem bloqueio.
    block_until: Option<u64>,
}

impl FailState {
    /// Estado vazio.
    const fn new() -> Self {
        Self {
            failures: VecDeque::new(),
            block_until: None,
        }
    }
}

/// Limitador de falhas de autenticação por IP.
#[derive(Debug)]
pub struct AuthLimiter {
    /// Falhas que disparam o bloqueio.
    max: u32,
    /// Janela de contagem das falhas (s).
    window_secs: u64,
    /// Duração do bloqueio (s).
    block_secs: u64,
    map: HashMap<IpAddr, FailState>,
}

impl AuthLimiter {
    /// Cria o limitador com os limites dados.
    #[must_use]
    pub fn new(max: u32, window_secs: u64, block_secs: u64) -> Self {
        Self {
            max,
            window_secs,
            block_secs,
            map: HashMap::new(),
        }
    }

    /// `true` se `ip` está bloqueado agora. Limpa entradas antigas.
    #[must_use]
    pub fn is_blocked(&mut self, ip: IpAddr, now: u64) -> bool {
        self.prune(now);
        self.map
            .get(&ip)
            .is_some_and(|st| st.block_until.is_some_and(|until| now < until))
    }

    /// Registra uma falha de autenticação de `ip`.
    ///
    /// Retorna `true` se **esta** falha acabou de colocar o IP em
    /// bloqueio (o chamador emite `AuthBlocked`). Falhas durante um
    /// bloqueio em curso não contam nem estendem o bloqueio.
    pub fn record_failure(&mut self, ip: IpAddr, now: u64) -> bool {
        self.prune(now);
        let st = self.map.entry(ip).or_insert_with(FailState::new);
        // Bloqueio em curso: mantém (sem contagem, sem extensão).
        if st.block_until.is_some_and(|until| now < until) {
            return false;
        }
        let cutoff = now.saturating_sub(self.window_secs);
        while st.failures.front().is_some_and(|t| *t < cutoff) {
            st.failures.pop_front();
        }
        st.failures.push_back(now);
        if st.failures.len() as u32 >= self.max {
            st.block_until = Some(now + self.block_secs);
            true
        } else {
            false
        }
    }

    /// Autenticação bem-sucedida: zera o contador do IP.
    pub fn record_success(&mut self, ip: IpAddr) {
        self.map.remove(&ip);
    }

    /// Quantidade de IPs com estado (testes/monitoração).
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// `true` se nenhum IP tem estado.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Remove entradas sem estado ativo: bloqueio expirado sai do mapa;
    /// falhas fora da janela são descartadas.
    pub fn prune(&mut self, now: u64) {
        let cutoff = now.saturating_sub(self.window_secs);
        self.map.retain(|_, st| {
            if st.block_until.is_some_and(|until| now < until) {
                return true; // bloqueio em curso
            }
            st.block_until = None; // bloqueio expirou: janela reinicia
            st.failures.iter().any(|t| *t >= cutoff)
        });
    }
}

/// Limitador de taxa de novas conexões por IP.
#[derive(Debug)]
pub struct ConnRateLimiter {
    /// Conexões por IP permitidas dentro da janela.
    max: usize,
    /// Janela de contagem (s).
    window_secs: u64,
    map: HashMap<IpAddr, VecDeque<u64>>,
}

impl ConnRateLimiter {
    /// Cria o limitador com os limites dados.
    #[must_use]
    pub fn new(max: usize, window_secs: u64) -> Self {
        Self {
            max,
            window_secs,
            map: HashMap::new(),
        }
    }

    /// `true` = pode aceitar (a tentativa **permitida** é registrada;
    /// tentativas recusadas não enchem a própria janela).
    pub fn allow(&mut self, ip: IpAddr, now: u64) -> bool {
        self.prune(now);
        let cutoff = now.saturating_sub(self.window_secs);
        let entry = self.map.entry(ip).or_default();
        while entry.front().is_some_and(|t| *t < cutoff) {
            entry.pop_front();
        }
        if entry.len() >= self.max {
            return false;
        }
        entry.push_back(now);
        true
    }

    /// Quantidade de IPs com estado (testes/monitoração).
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// `true` se nenhum IP tem estado.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Remove IPs sem conexões dentro da janela.
    pub fn prune(&mut self, now: u64) {
        let cutoff = now.saturating_sub(self.window_secs);
        self.map.retain(|_, ts| {
            while ts.front().is_some_and(|t| *t < cutoff) {
                ts.pop_front();
            }
            !ts.is_empty()
        });
    }
}

/// Limitadores de segurança compartilhados pelo accept loop e pelo
/// handshake (F1-T15). Compartilhado como `Arc<Limiters>` (o clone é
/// do `Arc`; os `Mutex` internos são o estado único).
pub struct Limiters {
    /// Falhas de autenticação por IP (bloqueio temporário).
    pub auth: Mutex<AuthLimiter>,
    /// Taxa de novas conexões por IP (recusada na hora, sem bloqueio).
    pub conn_rate: Mutex<ConnRateLimiter>,
}
