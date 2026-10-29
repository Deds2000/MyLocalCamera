//! Áudio do mock: seno contínuo em PCM s16le (48 kHz mono).

/// Amostras por bloco de 10 ms a 48 kHz.
pub const BLOCK_SAMPLES: usize = 480;
/// Taxa do PCM do protocolo (`docs/CHEATSHEET.md`).
pub const SAMPLE_RATE: u32 = 48_000;
/// Duração de um bloco, em µs (usada no `pts`).
pub const BLOCK_PTS_US: u64 = 10_000;

/// Gerador de seno com fase acumulada entre blocos (sem saltos).
pub struct SineGen {
    phase: f32,
    freq: f32,
    rate: u32,
}

impl SineGen {
    /// Gerador de `freq` Hz na taxa `rate`.
    #[must_use]
    pub fn new(freq: f32, rate: u32) -> Self {
        Self {
            phase: 0.0,
            freq,
            rate,
        }
    }

    /// Próximo bloco de `n_samples` amostras s16le (amplitude 0,5).
    pub fn next_block(&mut self, n_samples: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(n_samples * 2);
        for _ in 0..n_samples {
            let sample = (self.phase * std::f32::consts::TAU).sin();
            self.phase = (self.phase + self.freq / self.rate as f32).rem_euclid(1.0);
            let value = (sample * f32::from(i16::MAX) * 0.5) as i16;
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{SineGen, BLOCK_SAMPLES, SAMPLE_RATE};

    /// UT `sine_is_continuous`: 100 blocos (1 s) sem salto de fase e
    /// frequência ≈ 440 Hz (contagem de cruzamentos de zero).
    #[test]
    fn sine_is_continuous() {
        let mut gen = SineGen::new(440.0, SAMPLE_RATE);
        let mut samples = Vec::with_capacity(BLOCK_SAMPLES * 100);
        for _ in 0..100 {
            let block = gen.next_block(BLOCK_SAMPLES);
            assert_eq!(block.len(), BLOCK_SAMPLES * 2);
            for pair in block.as_chunks::<2>().0 {
                samples.push(i16::from_le_bytes(*pair));
            }
        }
        assert_eq!(samples.len(), SAMPLE_RATE as usize);

        let mut max_step = 0i32;
        let mut crossings = 0u32;
        let mut last_sign = samples[0] >= 0;
        for pair in samples.windows(2) {
            let step = (i32::from(pair[1]) - i32::from(pair[0])).abs();
            max_step = max_step.max(step);
            let sign = pair[1] >= 0;
            if sign != last_sign {
                crossings += 1;
                last_sign = sign;
            }
        }
        let freq = f32::from(u16::try_from(crossings).expect("contagem cabe")) / 2.0;
        assert!((freq - 440.0).abs() <= 2.0, "frequência medida {freq} Hz");
        assert!(max_step < 1_000, "salto de fase entre amostras: {max_step}");
    }
}
