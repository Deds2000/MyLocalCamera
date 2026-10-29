//! Estatísticas agregadas da sessão (F1-T10): a função pura
//! [`compute_stats`] (diferenças de contadores entre dois snapshots) e a
//! task por sessão que, a cada `stats_interval`, lê os
//! [`SessionCounters`], chama `compute_stats` e emite
//! [`ServerEvent::Stats`] — a base dos 2 Hz da UI.
//!
//! Índices dos vetores por stream: `0` = stream 1, `1` = stream 2,
//! `2` = reservado.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};

use crate::events::ServerEvent;
use crate::sessions::SessionCounters;

/// Snapshot dos contadores da sessão (leitura instantânea sem lock).
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Snapshot {
    /// Quadros de vídeo recebidos, por stream.
    pub video_frames: [u64; 3],
    /// Bytes de vídeo recebidos, por stream.
    pub video_bytes: [u64; 3],
    /// Blocos de áudio recebidos.
    pub audio_blocks: u64,
    /// Bytes de áudio recebidos.
    pub audio_bytes: u64,
}

/// Estatísticas agregadas de um intervalo de `stats_interval`.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionStats {
    /// Quadros/s, por stream.
    pub video_fps: [f32; 3],
    /// Kbits/s, por stream.
    pub video_kbps: [f32; 3],
    /// Blocos de áudio/s.
    pub audio_blocks_per_s: f32,
    /// RTT (EWMA) do socket principal, em ms; `None` sem medição ainda.
    pub rtt_ms: Option<f32>,
    /// Totais acumulados da sessão (fim do intervalo).
    pub totals: Snapshot,
}

/// Converte um contador `u64` em `f32` (exato até 2²⁴; acima disso,
/// arredondado — irrelevante para taxas de vídeo/áudio).
#[must_use]
fn as_f32(v: u64) -> f32 {
    v as f32
}

/// Computa as estatísticas do intervalo `[prev, curr]` com duração `dt`.
///
/// - `dt == 0` → taxas zeradas (sem divisão por zero);
/// - `curr < prev` em uma métrica → taxa zero daquela métrica (sessão
///   nova zerou os contadores; sem underflow).
#[must_use]
pub fn compute_stats(
    prev: &Snapshot,
    curr: &Snapshot,
    dt: Duration,
    rtt_ms: Option<f32>,
) -> SessionStats {
    let dt_s = dt.as_secs_f32();
    let rates = dt_s > 0.0;
    let mut video_fps = [0.0f32; 3];
    let mut video_kbps = [0.0f32; 3];
    if rates {
        for i in 0..3 {
            let frames = curr.video_frames[i].saturating_sub(prev.video_frames[i]);
            let bytes = curr.video_bytes[i].saturating_sub(prev.video_bytes[i]);
            video_fps[i] = as_f32(frames) / dt_s;
            video_kbps[i] = as_f32(bytes) * 8.0 / 1_000.0 / dt_s;
        }
    }
    let audio_blocks_per_s = if rates {
        let diff = curr.audio_blocks.saturating_sub(prev.audio_blocks);
        as_f32(diff) / dt_s
    } else {
        0.0
    };
    SessionStats {
        video_fps,
        video_kbps,
        audio_blocks_per_s,
        rtt_ms,
        totals: *curr,
    }
}

/// Task agregadora por sessão (F1-T10): a cada `stats_interval` lê os
/// contadores, chama [`compute_stats`] e emite `ServerEvent::Stats`.
/// Termina quando `done_rx` dispara (fim da sessão) ou a fila de eventos
/// fecha. O RTT vem do `SessionCounters` (gravado pelo laço principal).
pub(crate) async fn session_stats_task(
    session_id: String,
    counters: Arc<SessionCounters>,
    stats_interval: Duration,
    events_tx: mpsc::Sender<ServerEvent>,
    mut done_rx: oneshot::Receiver<()>,
) {
    let mut prev = counters.snapshot();
    let mut last = tokio::time::Instant::now();
    let mut interval = tokio::time::interval(stats_interval);
    interval.tick().await; // descarta o tick imediato (1ª emissão após 1 ciclo)
    loop {
        tokio::select! {
            _ = &mut done_rx => return,
            _ = interval.tick() => {}
        }
        let now = tokio::time::Instant::now();
        let dt = now.saturating_duration_since(last);
        last = now;
        let curr = counters.snapshot();
        let rtt = counters.rtt_ewma_ms.load(Ordering::Relaxed);
        let rtt_ms = (rtt != 0).then_some(f64::from(rtt) as f32);
        let stats = compute_stats(&prev, &curr, dt, rtt_ms);
        prev = curr;
        if events_tx
            .send(ServerEvent::Stats {
                session_id: session_id.clone(),
                stats,
            })
            .await
            .is_err()
        {
            return; // fila fechou: nada a emitir
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UT `compute_stats_basic`: 30 quadros e 1 MB em 1 s → 30 fps,
    /// 8000 kbps.
    #[test]
    fn compute_stats_basic() {
        let prev = Snapshot::default();
        let curr = Snapshot {
            video_frames: [30, 0, 0],
            video_bytes: [1_000_000, 0, 0],
            audio_blocks: 0,
            audio_bytes: 0,
        };
        let s = compute_stats(&prev, &curr, Duration::from_secs(1), None);
        assert_eq!(s.video_fps[0], 30.0, "30 quadros em 1 s = 30 fps");
        assert!(
            (s.video_kbps[0] - 8_000.0).abs() < 1e-6,
            "1 MB em 1 s = 8000 kbps; veio {}",
            s.video_kbps[0]
        );
        assert_eq!(s.video_fps[1], 0.0, "stream 2 sem quadros");
        assert_eq!(s.totals, curr, "totais = snapshot atual");
    }

    /// UT `compute_stats_dt_zero`: `dt = 0` → zeros, sem NaN (sem
    /// divisão por zero).
    #[test]
    fn compute_stats_dt_zero() {
        let curr = Snapshot {
            video_frames: [30, 0, 0],
            ..Snapshot::default()
        };
        let s = compute_stats(&Snapshot::default(), &curr, Duration::ZERO, None);
        assert_eq!(s.video_fps, [0.0; 3], "dt=0 → fps zerado");
        assert_eq!(s.video_kbps, [0.0; 3], "dt=0 → kbps zerado");
        assert_eq!(s.audio_blocks_per_s, 0.0, "dt=0 → áudio zerado");
        assert!(
            s.video_fps
                .iter()
                .chain(s.video_kbps.iter())
                .all(|v| !v.is_nan()),
            "sem NaN"
        );
        assert_eq!(s.totals, curr, "totais seguem corretos");
    }

    /// UT `compute_stats_counters_reset`: `curr < prev` (sessão nova
    /// zerou os contadores) → zeros, sem underflow.
    #[test]
    fn compute_stats_counters_reset() {
        let prev = Snapshot {
            video_frames: [100, 50, 0],
            video_bytes: [10_000, 5_000, 0],
            audio_blocks: 90,
            audio_bytes: 1_000,
        };
        let curr = Snapshot::default();
        let s = compute_stats(&prev, &curr, Duration::from_secs(1), None);
        assert_eq!(s.video_fps, [0.0; 3], "reset → zero (sem underflow)");
        assert_eq!(s.video_kbps, [0.0; 3], "reset → zero (sem underflow)");
        assert_eq!(s.audio_blocks_per_s, 0.0, "reset → zero (sem underflow)");
        assert_eq!(s.totals, curr, "totais = snapshot zerado");
    }
}
