//! Tasks de mídia do mock: leitura (PING→PONG), vídeo e áudio.
//!
//! Extraído de `session.rs` para manter os arquivos dentro do limite de
//! ~300 linhas (o cartão F1-T11a fixa `session.rs` para a orquestração).

use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use bytes::Bytes;
use lc_protocol::json::AudioHello;
use lc_protocol::Message;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::{interval, MissedTickBehavior};

use crate::audio::{SineGen, BLOCK_PTS_US, BLOCK_SAMPLES, SAMPLE_RATE};
use crate::video;
use crate::{send_msg, Counters, MsgReader, Writer};

/// Leitura da sessão: responde `PING` e guarda o motivo do `BYE`.
pub(crate) async fn reader_loop(
    mut msg_reader: MsgReader,
    writer: Writer,
    closed_by: Arc<StdMutex<Option<String>>>,
) {
    loop {
        match msg_reader.next().await {
            Ok(Some(Message::Ping(value))) => {
                if send_msg(&writer, &Message::Pong(value)).await.is_err() {
                    break;
                }
            }
            Ok(Some(Message::Bye(bye))) => {
                *closed_by.lock().unwrap_or_else(|p| p.into_inner()) = Some(bye.reason);
                break;
            }
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => break,
        }
    }
}

/// Envia os access units a `fps`, com `pts` crescente; `loop` reinicia.
pub(crate) async fn video_loop(
    data: Vec<u8>,
    aus: Vec<std::ops::Range<usize>>,
    fps: u32,
    loop_video: bool,
    writer: Writer,
    counters: Arc<Counters>,
) {
    let mut ticker = interval(Duration::from_secs_f64(1.0 / f64::from(fps)));
    // `Burst`: o timer do Windows tem granularidade de ~15,6 ms; com
    // `Delay` um intervalo de 33,3 ms viraria ~46,8 ms e o vídeo cairia
    // para ~21 fps. O burst compensa os ticks atrasados sem mudar a média.
    ticker.set_missed_tick_behavior(MissedTickBehavior::Burst);
    ticker.tick().await; // descarta o tick imediato
    let mut index = 0u64;
    'outer: loop {
        for range in &aus {
            let au = &data[range.clone()];
            let flags = video::flags_from_au(au);
            let pts = video::pts_for_frame(index, fps);
            let msg = Message::Video {
                stream: 1,
                flags,
                pts_us: pts,
                data: Bytes::copy_from_slice(au),
            };
            if send_msg(&writer, &msg).await.is_err() {
                break 'outer;
            }
            counters.video_frames.fetch_add(1, Ordering::Relaxed);
            counters
                .video_bytes
                .fetch_add(au.len() as u64, Ordering::Relaxed);
            index += 1;
            ticker.tick().await;
        }
        if !loop_video {
            break;
        }
    }
}

/// Abre o socket de áudio, espera o `WELCOME` e envia blocos de 10 ms.
pub(crate) async fn audio_loop(
    addr: SocketAddr,
    session_id: String,
    ticket: String,
    tone: f32,
    counters: Arc<Counters>,
) -> Result<(), crate::session::MockError> {
    let stream = TcpStream::connect(addr).await?;
    let (read_half, write_half) = stream.into_split();
    let writer: Writer = Arc::new(Mutex::new(write_half));
    send_msg(
        &writer,
        &Message::AudioHello(AudioHello { session_id, ticket }),
    )
    .await?;

    let mut msg_reader = MsgReader::new(read_half);
    loop {
        match msg_reader.next().await? {
            Some(Message::Welcome(welcome)) if welcome.accepted => break,
            Some(Message::Welcome(welcome)) => {
                return Err(crate::session::MockError::Denied(welcome.reason));
            }
            Some(_) => {}
            None => return Err(crate::session::MockError::Closed),
        }
    }
    tracing::info!("socket de áudio aberto");

    let mut ticker = interval(Duration::from_millis(10));
    // Mesmo motivo do vídeo: `Burst` compensa a granularidade do timer
    // do Windows (~15,6 ms) e mantém a média de 100 blocos/s.
    ticker.set_missed_tick_behavior(MissedTickBehavior::Burst);
    ticker.tick().await;
    let mut generator = SineGen::new(tone, SAMPLE_RATE);
    let mut pts = 0u64;
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let block = generator.next_block(BLOCK_SAMPLES);
                let msg = Message::Audio {
                    pts_us: pts,
                    data: Bytes::from(block),
                };
                send_msg(&writer, &msg).await?;
                counters.audio_blocks.fetch_add(1, Ordering::Relaxed);
                pts += BLOCK_PTS_US;
            }
            received = msg_reader.next() => match received {
                Ok(Some(Message::Ping(value))) => {
                    send_msg(&writer, &Message::Pong(value)).await?;
                }
                Ok(Some(Message::Bye(_))) | Ok(None) | Err(_) => break,
                Ok(Some(_)) => {}
            }
        }
    }
    Ok(())
}
