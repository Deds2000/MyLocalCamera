//! Sessão do mock: HELLO/WELCOME, orquestração das tasks de mídia.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use lc_protocol::json::{
    AppInfo, Auth, Caps, DeviceInfo, Hello, ProtocolVersion, Reason, StreamConfig, Welcome,
};
use lc_protocol::Message;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::state_file::{self, State};
use crate::video;
use crate::{media, send_msg, Counters, MsgReader, Writer};

/// Configuração resolvida da execução.
#[derive(Debug, Clone)]
pub struct MockConfig {
    pub hosts: Vec<String>,
    pub port: u16,
    pub token: Option<String>,
    pub device_id: String,
    pub device_name: String,
    pub device_key: Option<String>,
    pub state_file: Option<PathBuf>,
    pub file: Option<PathBuf>,
    pub loop_video: bool,
    pub fps: u32,
    pub audio: bool,
    pub tone: f32,
    pub seconds: Option<f64>,
}

/// Resultado de uma execução.
#[derive(Debug, Clone)]
pub struct Summary {
    pub video_frames: u64,
    pub video_bytes: u64,
    pub audio_blocks: u64,
    pub closed_by: Option<String>,
    pub session_id: String,
}

/// Falhas do mock.
#[derive(Debug, thiserror::Error)]
pub enum MockError {
    /// Servidor recusou o pareamento/autenticação.
    #[error("servidor recusou: {0:?}")]
    Denied(Option<Reason>),
    /// Nenhum host da lista aceitou a conexão.
    #[error("nenhum host respondeu ({0})")]
    NoHost(String),
    /// Conexão caiu antes do `WELCOME`.
    #[error("conexão encerrada sem WELCOME")]
    Closed,
    /// `WELCOME` aceito sem `sessionId`.
    #[error("WELCOME sem sessionId")]
    MissingSession,
    /// Áudio pedido, mas o `WELCOME` não trouxe `audioTicket`.
    #[error("WELCOME sem audioTicket")]
    MissingAudioTicket,
    /// Configuração incompleta (sem token nem `deviceKey`).
    #[error("configuração: {0}")]
    Config(String),
    /// Arquivo de vídeo inválido.
    #[error("vídeo: {0}")]
    Video(String),
    /// Erro de rede/IO.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Executa o mock até o fim do vídeo, o prazo de `--seconds` ou Ctrl+C.
///
/// # Errors
/// Recusa do servidor, host inacessível, arquivo inválido ou erro de I/O.
pub async fn run(cfg: MockConfig) -> Result<Summary, MockError> {
    let (stream, addr) = connect_first(&cfg.hosts, cfg.port).await?;
    let _ = stream.set_nodelay(true);

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
    let hello = Message::Hello(Hello {
        protocol: ProtocolVersion { major: 1, minor: 0 },
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

    let (read_half, write_half) = stream.into_split();
    let writer: Writer = Arc::new(Mutex::new(write_half));
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
        Some(tokio::spawn(async move {
            if let Err(e) = media::audio_loop(addr, audio_session, ticket, tone, counters).await {
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
    tokio::select! {
        () = deadline => {}
        () = wait_handle(&mut reader_task) => {}
        () = wait_handle(&mut video_task) => {}
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

    let closed_by = closed_by.lock().unwrap_or_else(|p| p.into_inner()).clone();
    Ok(Summary {
        video_frames: counters.video_frames(),
        video_bytes: counters.video_bytes(),
        audio_blocks: counters.audio_blocks(),
        closed_by,
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
