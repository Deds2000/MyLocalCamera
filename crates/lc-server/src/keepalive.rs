//! Keepalive (F1-T10): `PING` periódico, RTT por média móvel exponencial
//! e detecção de conexão perdida (sem `PONG` por `liveness_timeout`).
//!
//! O payload do `PING` é o relógio monotônico em ms ([`mono_ms`]); o
//! cliente devolve `PONG` com o **mesmo** valor e o RTT =
//! `agora_ms - valor_do_pong`. **Nenhum** outro tráfego substitui o
//! `PONG` (regra do cartão F1-T10).

use std::time::{Duration, Instant};

/// Peso da média móvel exponencial do RTT (F1-T10: α = 0,3).
pub const RTT_ALPHA: f32 = 0.3;

/// Estado de keepalive de um socket (principal ou áudio).
///
/// `last_pong` começa no instante da criação (a sessão acaba de abrir):
/// o primeiro `PONG` precisa chegar dentro de `liveness_timeout` da
/// abertura do socket.
#[derive(Debug)]
pub struct Keepalive {
    /// Instante do último `PONG` (ou da criação, se ainda não houve).
    last_pong: Instant,
    /// RTT (EWMA, α = [`RTT_ALPHA`]) em ms; `None` antes do 1º `PONG`.
    rtt_ewma_ms: Option<f32>,
    /// Sem `PONG` por este intervalo → conexão perdida.
    liveness_timeout: Duration,
}

impl Keepalive {
    /// Keepalive novo: o "último PONG" é agora.
    #[must_use]
    pub fn new(liveness_timeout: Duration) -> Self {
        Self::at(Instant::now(), liveness_timeout)
    }

    /// Keepalive com `last_pong` explícito (relógio virtual nos testes —
    /// `Instant` suporta aritmética sem depender do relógio real).
    #[must_use]
    pub fn at(last_pong: Instant, liveness_timeout: Duration) -> Self {
        Self {
            last_pong,
            rtt_ewma_ms: None,
            liveness_timeout,
        }
    }

    /// Registra um `PONG`: `sent_ms` é o valor enviado no `PING` e
    /// `now_ms` o relógio monotônico agora (ms). Atualiza o RTT (EWMA).
    pub fn on_pong(&mut self, sent_ms: u64, now_ms: u64) {
        self.last_pong = Instant::now();
        let rtt = now_ms.saturating_sub(sent_ms) as f32;
        self.rtt_ewma_ms = Some(match self.rtt_ewma_ms {
            None => rtt,
            Some(prev) => prev + (rtt - prev) * RTT_ALPHA,
        });
    }

    /// RTT (EWMA) em ms, se já houve `PONG`.
    #[must_use]
    pub fn rtt_ms(&self) -> Option<f32> {
        self.rtt_ewma_ms
    }

    /// `true` se sem `PONG` por **mais** que `liveness_timeout` em `now`.
    #[must_use]
    pub fn is_dead(&self, now: Instant) -> bool {
        now.duration_since(self.last_pong) > self.liveness_timeout
    }

    /// Prazo do liveness: `last_pong + liveness_timeout` (para
    /// `sleep_until` no laço da conexão).
    #[must_use]
    pub fn deadline(&self) -> Instant {
        self.last_pong + self.liveness_timeout
    }
}

/// Relógio monotônico do processo, em ms (payload dos `PING`).
///
/// Base fixa no 1º uso (`OnceLock`): `now - base`.
#[must_use]
pub fn mono_ms() -> u64 {
    static BASE: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let base = BASE.get_or_init(Instant::now);
    let ms = Instant::now().duration_since(*base).as_millis();
    u64::try_from(ms).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UT `keepalive_dead_after_timeout` (relógio virtual): morto só
    /// após 6 s sem `PONG`.
    #[test]
    fn keepalive_dead_after_timeout() {
        let t0 = Instant::now();
        let ka = Keepalive::at(t0, Duration::from_secs(6));
        assert!(!ka.is_dead(t0), "recém-criado não está morto");
        assert!(!ka.is_dead(t0 + Duration::from_secs(5)));
        assert!(!ka.is_dead(t0 + Duration::from_millis(5_999)));
        assert!(
            ka.is_dead(t0 + Duration::from_millis(6_001)),
            "após 6 s está morto"
        );
    }

    /// UT `rtt_ewma`: PONGs de 50, 50, 100 ms → valor entre 50 e 100.
    #[test]
    fn rtt_ewma() {
        let mut ka = Keepalive::at(Instant::now(), Duration::from_secs(6));
        ka.on_pong(0, 50);
        ka.on_pong(100, 150); // 50 ms
        ka.on_pong(200, 300); // 100 ms
        let rtt = ka.rtt_ms().expect("RTT medido");
        assert!(
            (50.0..=100.0).contains(&rtt),
            "EWMA de 50/50/100 deve ficar em [50, 100], veio {rtt}"
        );
        assert!(
            (rtt - 65.0).abs() < 0.1,
            "50 → 50 → 65 (α = 0,3); veio {rtt}"
        );
    }

    /// `on_pong` renova o liveness (relógio real do `now`): depois do
    /// PONG, um ponto a 5 s (dentro do timeout de 6 s) não é mais
    /// "morto".
    #[test]
    fn pong_resets_deadline() {
        let t0 = Instant::now();
        let mut ka = Keepalive::at(t0, Duration::from_secs(6));
        assert!(ka.is_dead(t0 + Duration::from_secs(7)));
        ka.on_pong(0, 7_000);
        assert!(
            !ka.is_dead(t0 + Duration::from_secs(5)),
            "PONG renovou o liveness"
        );
    }
}
