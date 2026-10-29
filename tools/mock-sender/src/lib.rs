//! `mock-sender`: finge ser o celular LocalCamera (F1-T11a).
//!
//! Pareia (URI/`--host`), reconecta com a chave salva, envia vídeo H.264
//! em loop com *pacing* e áudio (seno) pelo socket de áudio.

#![forbid(unsafe_code)]

pub mod audio;
pub mod cli;
pub mod config;
pub mod faults;
pub mod media;
pub mod session;
pub mod state_file;
pub mod uri;
pub mod video;

use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use faults::FaultyWriter;
use lc_protocol::{FrameReader, Message};
use tokio::io::AsyncReadExt;
use tokio::net::tcp::OwnedReadHalf;
use tokio::sync::Mutex;

/// Metade de escrita serializada (PONG, vídeo e pedidos dividem o socket).
pub(crate) type Writer = Arc<Mutex<FaultyWriter>>;

/// Envia uma mensagem completa pelo socket (encode + flush).
pub(crate) async fn send_msg(writer: &Writer, msg: &Message) -> io::Result<()> {
    let bytes = msg
        .encode(0)
        .map_err(|e| io::Error::other(format!("encode: {e}")))?;
    send_raw(writer, &bytes).await
}

/// Envia bytes crus (quadros malformados ou de tipo desconhecido).
pub(crate) async fn send_raw(writer: &Writer, bytes: &[u8]) -> io::Result<()> {
    let mut guard = writer.lock().await;
    guard.write_bytes(bytes).await
}

/// Leitor de mensagens com fila: um `read` pode trazer vários quadros e
/// nenhum pode ser perdido (um `PING` descartado derrubaria a sessão).
pub(crate) struct MsgReader {
    read: OwnedReadHalf,
    reader: FrameReader,
    buf: Vec<u8>,
    queue: VecDeque<Message>,
}

impl MsgReader {
    pub(crate) fn new(read: OwnedReadHalf) -> Self {
        Self {
            read,
            reader: FrameReader::new(),
            buf: vec![0u8; 64 * 1024],
            queue: VecDeque::new(),
        }
    }

    /// Próxima mensagem; `None` em EOF. Quadros de tipo desconhecido ou
    /// inválidos são ignorados (o mock não derruba a sessão por isso).
    pub(crate) async fn next(&mut self) -> io::Result<Option<Message>> {
        loop {
            if let Some(msg) = self.queue.pop_front() {
                return Ok(Some(msg));
            }
            let n = self.read.read(&mut self.buf).await?;
            if n == 0 {
                return Ok(None);
            }
            let frames = self
                .reader
                .push(&self.buf[..n])
                .map_err(|e| io::Error::other(format!("frame inválido: {e}")))?;
            for frame in frames {
                if let Ok(msg) = Message::from_frame(&frame) {
                    self.queue.push_back(msg);
                }
            }
        }
    }
}

/// Contadores compartilhados das tasks de mídia.
#[derive(Default)]
pub(crate) struct Counters {
    pub video_frames: AtomicU64,
    pub video_bytes: AtomicU64,
    pub audio_blocks: AtomicU64,
}

impl Counters {
    pub(crate) fn video_frames(&self) -> u64 {
        self.video_frames.load(Ordering::Relaxed)
    }

    pub(crate) fn video_bytes(&self) -> u64 {
        self.video_bytes.load(Ordering::Relaxed)
    }

    pub(crate) fn audio_blocks(&self) -> u64 {
        self.audio_blocks.load(Ordering::Relaxed)
    }
}
