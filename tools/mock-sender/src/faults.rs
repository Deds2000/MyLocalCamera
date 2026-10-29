//! Modos de falha determinísticos do `mock-sender` (F1-T11b).
//!
//! [`FaultyWriter`] envolve a metade de escrita do socket e aplica
//! fragmentação, limite de vazão (token bucket por tempo) e parada
//! temporária. Os quadros malformados são montados aqui.

use std::io;
use std::time::{Duration, Instant};

use lc_protocol::frame::encode_frame;
use lc_protocol::types::{HEADER_LEN, MAGIC, MAX_PAYLOAD};
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::time::sleep;

/// Conjunto de falhas pedidas na CLI. Tudo determinístico.
#[derive(Debug, Clone, Default)]
pub struct Faults {
    /// Todos os `write` saem em pedaços de N bytes (com `flush` entre eles).
    pub fragment: Option<usize>,
    /// Primeiro quadro com `magic` errado.
    pub bad_magic: bool,
    /// Primeiro quadro com `length = 8 MiB + 1` (só o cabeçalho).
    pub huge_length: bool,
    /// `HELLO` com JSON inválido.
    pub bad_json: bool,
    /// Insere um quadro de tipo `0x99` a cada 30 quadros de vídeo.
    pub unknown_type: bool,
    /// Espera este tempo antes do `HELLO`.
    pub delay_hello: Option<Duration>,
    /// Não responde `PING`.
    pub no_pong: bool,
    /// Descarta o primeiro `PING` e responde os demais.
    pub drop_ping: bool,
    /// Limita a vazão de escrita a N kbit/s.
    pub slow_kbps: Option<u32>,
    /// `(pausa, após)`: depois de `após`, para de escrever por `pausa`.
    pub stall: Option<(Duration, Duration)>,
    /// Fecha o socket de forma abrupta (sem `BYE`) depois deste tempo.
    pub kill_after: Option<Duration>,
    /// Conecta/envia 1 s/desconecta, N vezes (usa `--state-file`).
    pub reconnect: Option<u32>,
    /// Versão de protocolo anunciada no `HELLO`.
    pub fake_version: Option<(u16, u16)>,
}

impl Faults {
    /// Algum modo de "primeiro quadro malicioso" está ativo?
    #[must_use]
    pub fn malformed_first_frame(&self) -> bool {
        self.bad_magic || self.huge_length || self.bad_json
    }

    /// Bytes do primeiro quadro malformado (o primeiro modo ativo vence).
    ///
    /// # Panics
    /// Só se `encode_frame` recusar um payload fixo minúsculo (impossível).
    #[must_use]
    pub fn first_frame_bytes(&self) -> Vec<u8> {
        if self.bad_magic {
            header(0x5858, lc_protocol::types::msg::HELLO, 0)
        } else if self.huge_length {
            header(MAGIC, lc_protocol::types::msg::HELLO, MAX_PAYLOAD + 1)
        } else {
            encode_frame(lc_protocol::types::msg::HELLO, 0, 0, 0, b"{isto nao e json")
                .expect("payload fixo cabe no limite")
        }
    }
}

/// Cabeçalho de 18 bytes com o magic e o length pedidos (sem payload).
fn header(magic: u16, msg_type: u8, len: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN);
    out.extend_from_slice(&magic.to_le_bytes());
    out.push(msg_type);
    out.push(0);
    out.push(0);
    out.push(0);
    out.extend_from_slice(&0u64.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out
}

/// Quadro de tipo desconhecido (`0x99`) com 3 bytes de payload.
#[must_use]
pub fn unknown_type_frame() -> Vec<u8> {
    encode_frame(0x99, 0, 0, 0, b"xyz").expect("payload fixo cabe no limite")
}

/// Limite de vazão por tempo: `sent` bytes levam `sent / taxa` desde o início.
struct Rate {
    start: Instant,
    sent: u64,
    bytes_per_sec: f64,
}

impl Rate {
    /// Espera o necessário para que o total (incluindo `n`) respeite a taxa.
    async fn throttle(&mut self, n: usize) {
        self.sent += n as u64;
        let target = Duration::from_secs_f64(self.sent as f64 / self.bytes_per_sec);
        let elapsed = self.start.elapsed();
        if target > elapsed {
            sleep(target - elapsed).await;
        }
    }
}

/// Parada única: depois de `after`, dorme `pause` na primeira escrita.
struct Stall {
    start: Instant,
    after: Duration,
    pause: Duration,
    done: bool,
}

/// Metade de escrita com as falhas aplicadas. Todo envio do mock passa aqui.
pub struct FaultyWriter {
    write: OwnedWriteHalf,
    fragment: Option<usize>,
    rate: Option<Rate>,
    stall: Option<Stall>,
}

impl FaultyWriter {
    /// Envolve a metade de escrita; `now` marca o início do `stall`/`slow`.
    #[must_use]
    pub fn new(write: OwnedWriteHalf, faults: &Faults) -> Self {
        Self {
            write,
            fragment: faults.fragment.map(|n| n.max(1)),
            rate: faults.slow_kbps.filter(|k| *k > 0).map(|kbps| Rate {
                start: Instant::now(),
                sent: 0,
                bytes_per_sec: f64::from(kbps) * 1_000.0 / 8.0,
            }),
            stall: faults.stall.map(|(pause, after)| Stall {
                start: Instant::now(),
                after,
                pause,
                done: false,
            }),
        }
    }

    /// Escreve todos os bytes aplicando stall, vazão e fragmentação.
    ///
    /// # Errors
    /// Erros de escrita/flush do socket.
    pub async fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        if let Some(stall) = &mut self.stall {
            if !stall.done && stall.start.elapsed() >= stall.after {
                stall.done = true;
                sleep(stall.pause).await;
            }
        }
        if let Some(rate) = &mut self.rate {
            rate.throttle(bytes.len()).await;
        }
        match self.fragment {
            Some(n) => {
                for chunk in bytes.chunks(n) {
                    self.write.write_all(chunk).await?;
                    self.write.flush().await?;
                }
            }
            None => {
                self.write.write_all(bytes).await?;
                self.write.flush().await?;
            }
        }
        Ok(())
    }

    /// Fecha a escrita de forma abrupta (FIN, sem `BYE`).
    ///
    /// # Errors
    /// Erro de shutdown do socket.
    pub async fn shutdown(&mut self) -> io::Result<()> {
        self.write.shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::{header, unknown_type_frame, Faults};
    use lc_protocol::types::{HEADER_LEN, MAX_PAYLOAD};

    /// UT: o cabeçalho tem 18 bytes e o length vai no fim (LE).
    #[test]
    fn huge_header_layout() {
        let bytes = header(0x4C43, 1, MAX_PAYLOAD + 1);
        assert_eq!(bytes.len(), HEADER_LEN);
        assert_eq!(&bytes[0..2], &[0x43, 0x4C]);
        assert_eq!(&bytes[14..18], &(MAX_PAYLOAD + 1).to_le_bytes());
    }

    /// UT: quadro desconhecido = 18 + 3 bytes.
    #[test]
    fn unknown_frame_len() {
        assert_eq!(unknown_type_frame().len(), HEADER_LEN + 3);
    }

    /// UT: `malformed_first_frame` reflete as flags.
    #[test]
    fn malformed_flag() {
        let mut faults = Faults::default();
        assert!(!faults.malformed_first_frame());
        faults.bad_json = true;
        assert!(faults.malformed_first_frame());
    }
}
