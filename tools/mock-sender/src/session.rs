//! Sessão do mock: HELLO/WELCOME, orquestração das tasks de mídia e
//! modos de falha (F1-T11b).

use std::net::SocketAddr;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use lc_protocol::json::{
    AppInfo, Auth, Caps, DeviceInfo, Hello, ProtocolVersion, StreamConfig, Welcome,
};
use lc_protocol::Message;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub use crate::config::{MockConfig, MockError, Summary};
use crate::faults::FaultyWriter;
use crate::state_file::{self, State};
use crate::video;
use crate::{media, send_msg, send_raw, Counters, MsgReader, Writer};

/// Executa o mock até o fim do vídeo, o prazo de `--seconds` ou Ctrl+C.
/// Com `--reconnect N`, repete N sessões de 1 s reutilizando o estado.
///
/// # Errors
/// Recusa do servidor, host inacessível, arquivo inválido ou erro de I/O.
pub async fn run(mut cfg: MockConfig) -> Result<Summary, MockError> {
    let Some(total) = cfg.faults.reconnect else {
        return run_once(cfg).await;
    };
    if total == 0 {
        return Err(MockError::Config("--reconnect deve ser >= 1".to_owned()));
    }
    let mut aggregate = Summary::default();
    for index in 0..total {
        let mut one = cfg.clone();
        one.seconds = Some(1.0);
        one.faults.reconnect = None;
        let summary = run_once(one).await?;
        aggregate.video_frames += summary.video_frames;
        aggregate.video_bytes += summary.video_bytes;
        aggregate.audio_blocks += summary.audio_blocks;
        aggregate.sessions += 1;
        aggregate.closed_by = summary.closed_by;
        aggregate.session_id = summary.session_id;
        // Depois do 1º pareamento, as sessões seguintes usam o deviceKey
        // salvo (o token de pareamento rotaciona).
        if let Some(path) = &cfg.state_file {
            if let Some(state) = state_file::load(path) {
                cfg.device_id = state.device_id;
                cfg.device_key = Some(state.device_key);
                cfg.token = None;
            }
        }
        tracing::info!(session = index + 1, total, "reconexão concluída");
    }
    Ok(aggregate)
}

/// Uma sessão (ou um primeiro quadro malformado).
async fn run_once(cfg: MockConfig) -> Result<Summary, MockError> {
    let (stream, addr) = connect_first(&cfg.hosts, cfg.port).await?;
    let _ = stream.set_nodelay(true);

    let (read_half, write_half) = stream.into_split();
    let writer: Writer = Arc::new(Mutex::new(FaultyWriter::new(write_half, &cfg.faults)));

    // Modos "primeiro quadro malicioso": envia e lê até o servidor fechar.
    if cfg.faults.malformed_first_frame() {
        let started = Instant::now();
        send_raw(&writer, &cfg.faults.first_frame_bytes()).await?;
        let mut msg_reader = MsgReader::new(read_half);
        let drain = async {
            while msg_reader.next().await?.is_some() {}
            Ok::<(), MockError>(())
        };
        match tokio::time::timeout(Duration::from_secs(5), drain).await {
            Ok(result) => result?,
            Err(_) => {
                return Err(MockError::Config(
                    "servidor não fechou após o quadro malformado".to_owned(),
                ));
            }
        }
        return Ok(Summary {
            sessions: 1,
            closed_by: Some("server".to_owned()),
            closed_after_ms: Some(u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)),
            ..Summary::default()
        });
    }

    if let Some(delay) = cfg.faults.delay_hello {
        tokio::time::sleep(delay).await;
    }

    let auth = match &cfg.device_key {
        Some(key) => Auth::Device {
            device_id: cfg.device_id.clone(),
            key: key.clone(),
        },
        None => Auth::Pair {
            token: cfg
                .token
                .clone()
                .ok_or_else(|| MockError::Config("sem token nem deviceKey".to_owned()))?,
        },
    };
    let (major, minor) = cfg.faults.fake_version.unwrap_or((1, 0));
    let hello = Message::Hello(Hello {
        protocol: ProtocolVersion { major, minor },
        app: AppInfo {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            platform: "android".to_owned(),
        },
        device: DeviceInfo {
            id: cfg.device_id.clone(),
            name: cfg.device_name.clone(),
            model: "Mock".to_owned(),
            android_api: 33,
        },
        auth,
        caps: Caps {
            audio: cfg.audio,
            codecs: vec!["avc1".to_owned()],
            ..Caps::default()
        },
    });
    send_msg(&writer, &hello).await?;

    let mut msg_reader = MsgReader::new(read_half);
    let welcome = wait_welcome(&mut msg_reader).await?;
    if !welcome.accepted {
        return Err(MockError::Denied(welcome.reason));
    }
    let session_id = welcome
        .session_id
        .clone()
        .ok_or(MockError::MissingSession)?;
    tracing::info!(session = %session_id, "sessão aberta");

    if let (Some(key), Some(path)) = (&welcome.device_key, cfg.state_file.as_ref()) {
        state_file::save(
            path,
            &State {
                device_id: cfg.device_id.clone(),
                device_key: key.clone(),
            },
        )?;
        tracing::info!(path = %path.display(), "deviceKey salvo");
    }

    let counters = Arc::new(Counters::default());
    let closed_by: Arc<StdMutex<Option<String>>> = Arc::new(StdMutex::new(None));

    let mut reader_task = Some(tokio::spawn(media::reader_loop(
        msg_reader,
        writer.clone(),
        closed_by.clone(),
        cfg.faults.no_pong,
        cfg.faults.drop_ping,
    )));

    let mut video_task = if let Some(path) = &cfg.file {
        let (data, aus) = video::load_frames(path).map_err(|e| MockError::Video(e.to_string()))?;
        send_msg(
            &writer,
            &Message::StreamConfig(StreamConfig {
                stream: 1,
                width: 1920,
                height: 1080,
                fps: cfg.fps,
                rotation_hint: 0,
                facing: "back".to_owned(),
                profile: None,
            }),
        )
        .await?;
        Some(tokio::spawn(media::video_loop(
            data,
            aus,
            cfg.fps,
            cfg.loop_video,
            writer.clone(),
            counters.clone(),
            cfg.faults.unknown_type,
        )))
    } else {
        None
    };

    let mut audio_task = if cfg.audio {
        let ticket = welcome
            .audio_ticket
            .clone()
            .ok_or(MockError::MissingAudioTicket)?;
        let counters = counters.clone();
        let tone = cfg.tone;
        let audio_session = session_id.clone();
        let faults = cfg.faults.clone();
        Some(tokio::spawn(async move {
            if let Err(e) =
                media::audio_loop(addr, audio_session, ticket, tone, counters, faults).await
            {
                tracing::warn!(error = %e, "socket de áudio encerrou com erro");
            }
        }))
    } else {
        None
    };

    let deadline = async {
        match cfg.seconds {
            Some(secs) => tokio::time::sleep(Duration::from_secs_f64(secs)).await,
            None => std::future::pending::<()>().await,
        }
    };
    let kill_after = async {
        match cfg.faults.kill_after {
            Some(delay) => tokio::time::sleep(delay).await,
            None => std::future::pending::<()>().await,
        }
    };
    let mut killed = false;
    tokio::select! {
        () = deadline => {}
        () = wait_handle(&mut reader_task) => {}
        () = wait_handle(&mut video_task) => {}
        () = kill_after => {
            // Abrupto: FIN na escrita, sem `BYE`.
            killed = true;
            let _ = writer.lock().await.shutdown().await;
        }
        _ = tokio::signal::ctrl_c() => {}
    }

    for handle in [&mut reader_task, &mut video_task, &mut audio_task]
        .into_iter()
        .flatten()
    {
        // O handle que venceu o `select!` já foi consumido pelo
        // `wait_handle`; repolarizá-lo seria inválido.
        if !handle.is_finished() {
            handle.abort();
            let _ = handle.await;
        }
    }

    let closed_by = if killed {
        Some("killed".to_owned())
    } else {
        closed_by.lock().unwrap_or_else(|p| p.into_inner()).clone()
    };
    Ok(Summary {
        video_frames: counters.video_frames(),
        video_bytes: counters.video_bytes(),
        audio_blocks: counters.audio_blocks(),
        sessions: 1,
        closed_by,
        closed_after_ms: None,
        session_id,
    })
}

/// Conecta no primeiro host que responder.
async fn connect_first(hosts: &[String], port: u16) -> Result<(TcpStream, SocketAddr), MockError> {
    let mut last = String::new();
    for host in hosts {
        match TcpStream::connect((host.as_str(), port)).await {
            Ok(stream) => {
                let addr = stream.peer_addr()?;
                return Ok((stream, addr));
            }
            Err(e) => last = format!("{host}:{port}: {e}"),
        }
    }
    Err(MockError::NoHost(last))
}

/// Aguarda o `WELCOME`, aceitando `PENDING` antes.
async fn wait_welcome(msg_reader: &mut MsgReader) -> Result<Welcome, MockError> {
    loop {
        match msg_reader.next().await? {
            Some(Message::Pending(pending)) => {
                tracing::info!(timeout_s = pending.timeout_s, "aguardando aprovação no PC");
            }
            Some(Message::Welcome(welcome)) => return Ok(welcome),
            Some(_) => {}
            None => return Err(MockError::Closed),
        }
    }
}

/// Espera a task terminar (ou fica pendente se ela não existir).
async fn wait_handle(handle: &mut Option<JoinHandle<()>>) {
    if let Some(task) = handle.as_mut() {
        let _ = task.await;
    } else {
        std::future::pending::<()>().await;
    }
}
