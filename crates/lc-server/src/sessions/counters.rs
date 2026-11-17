//! Contadores atômicos da sessão ativa.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Contadores da sessão (F1-T09b/F1-T10): o socket de áudio escreve
/// `audio_*`, o laço principal escreve `video_*`/`rtt_ewma_ms`; a
/// UI/`STATS` lêem sem lock (`AtomicU64`).
///
/// Índices dos vetores por stream: `0` = stream 1, `1` = stream 2,
/// `2` = reservado.
#[derive(Debug, Default)]
pub struct SessionCounters {
    /// Quadros de vídeo recebidos, por stream.
    pub video_frames: [AtomicU64; 3],
    /// Bytes de vídeo recebidos, por stream.
    pub video_bytes: [AtomicU64; 3],
    /// Keyframes recebidos (qualquer stream).
    pub keyframes: AtomicU64,
    /// Blocos de áudio recebidos.
    pub audio_blocks: AtomicU64,
    /// Bytes de áudio recebidos.
    pub audio_bytes: AtomicU64,
    /// `ptsUs` do último bloco de áudio.
    pub last_audio_pts: AtomicU64,
    /// RTT (EWMA) do socket principal, em ms; `0` = ainda não medido
    /// (F1-T10; o laço principal grava a cada `PONG`).
    pub rtt_ewma_ms: AtomicU32,
}

impl SessionCounters {
    /// Snapshot sem lock dos contadores (F1-T10): alimenta
    /// [`crate::stats::compute_stats`] a cada `stats_interval`.
    #[must_use]
    pub fn snapshot(&self) -> crate::stats::Snapshot {
        let [f0, f1, f2] = &self.video_frames;
        let [b0, b1, b2] = &self.video_bytes;
        crate::stats::Snapshot {
            video_frames: [
                f0.load(Ordering::Relaxed),
                f1.load(Ordering::Relaxed),
                f2.load(Ordering::Relaxed),
            ],
            video_bytes: [
                b0.load(Ordering::Relaxed),
                b1.load(Ordering::Relaxed),
                b2.load(Ordering::Relaxed),
            ],
            audio_blocks: self.audio_blocks.load(Ordering::Relaxed),
            audio_bytes: self.audio_bytes.load(Ordering::Relaxed),
        }
    }
}
