//! Integração do canal de quadros e comandos com uma sessão TCP real.

use std::collections::VecDeque;
use std::time::Duration;

use bytes::Bytes;
use lc_protocol::json::{
    AppInfo, Auth, Caps, DeviceInfo, Hello, ProtocolVersion, SetItem, SetPayload, Welcome,
};
use lc_protocol::{flags, Frame, FrameReader, Message};
use lc_server::config::ServerConfig;
use lc_server::events::ServerEvent;
use lc_server::frames::KeyframeRequest;
use lc_server::sessions::SessionCommandError;
use lc_server::{start, ServerHandle};
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn boot() -> (ServerHandle, u16, TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = ServerConfig::for_tests(dir.path().to_owned(), true);
    let mut server = start(config).await.expect("start server");
    let port = match server.events().recv().await.expect("Listening") {
        ServerEvent::Listening { port } => port,
        other => panic!("esperava Listening; veio {other:?}"),
    };
    (server, port, dir)
}

fn pair_token(server: &ServerHandle) -> String {
    server
        .trust()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .pair_token()
        .to_owned()
}

struct Peer {
    stream: TcpStream,
    reader: FrameReader,
    pending: VecDeque<Frame>,
    buffer: [u8; 4096],
}

impl Peer {
    async fn send(&mut self, message: &Message) {
        let pts_us = match message {
            Message::Video { pts_us, .. } | Message::Audio { pts_us, .. } => *pts_us,
            _ => 0,
        };
        self.stream
            .write_all(&message.encode(pts_us).expect("encode"))
            .await
            .expect("write message");
    }

    async fn recv(&mut self) -> Message {
        loop {
            if let Some(frame) = self.pending.pop_front() {
                return Message::from_frame(&frame).expect("valid server message");
            }
            let read = self.stream.read(&mut self.buffer).await.expect("read");
            assert_ne!(read, 0, "servidor encerrou antes da mensagem");
            self.pending
                .extend(self.reader.push(&self.buffer[..read]).expect("frame"));
        }
    }
}

async fn connect(port: u16, token: String) -> (Peer, Welcome) {
    let mut peer = Peer {
        stream: TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect"),
        reader: FrameReader::new(),
        pending: VecDeque::new(),
        buffer: [0; 4096],
    };
    peer.send(&Message::Hello(Hello {
        protocol: ProtocolVersion { major: 1, minor: 0 },
        app: AppInfo {
            version: "test".to_owned(),
            platform: "android".to_owned(),
        },
        device: DeviceInfo {
            id: "media-test".to_owned(),
            name: "Media test".to_owned(),
            model: "Mock".to_owned(),
            android_api: 33,
        },
        auth: Auth::Pair { token },
        caps: Caps::default(),
    }))
    .await;
    let welcome = match peer.recv().await {
        Message::Welcome(welcome) => welcome,
        other => panic!("esperava WELCOME; veio {other:?}"),
    };
    assert!(welcome.accepted, "handshake aceito: {welcome:?}");
    (peer, welcome)
}

/// Quadros entregues preservam a ordem, PTS, flags, bytes e sessionId.
#[tokio::test]
async fn video_frames_preserve_order_and_metadata() {
    let (mut server, port, _dir) = boot().await;
    let token = pair_token(&server);
    let mut source = server.video_frames();
    let (mut peer, welcome) = connect(port, token).await;
    let session_id = welcome.session_id.expect("session id");
    let expected = [
        (101, 0, vec![1, 2, 3]),
        (202, flags::KEYFRAME, vec![4, 5, 6, 7]),
        (303, 2, vec![8, 9]),
    ];
    for (pts_us, flags, data) in &expected {
        peer.send(&Message::Video {
            stream: 2,
            flags: *flags,
            pts_us: *pts_us,
            data: Bytes::from(data.clone()),
        })
        .await;
    }

    for (pts_us, flags, data) in expected {
        let frame = tokio::time::timeout(Duration::from_secs(1), source.recv())
            .await
            .expect("frame arrive sans timeout");
        assert_eq!(frame.session_id, session_id);
        assert_eq!(frame.stream, 2);
        assert_eq!(frame.flags, flags);
        assert_eq!(frame.pts_us, pts_us);
        assert_eq!(frame.data.as_ref(), data);
    }
    drop(peer);
    server.shutdown().await;
}

/// A leitura da sessão e a resposta PONG continuam mesmo sem consumidor.
#[tokio::test]
async fn pong_is_not_blocked_by_stalled_video_consumer() {
    let (mut server, port, _dir) = boot().await;
    let token = pair_token(&server);
    let mut source = server.video_frames(); // intencionalmente não chama recv()
    let (mut peer, _welcome) = connect(port, token).await;

    for pts_us in 0..32 {
        peer.send(&Message::Video {
            stream: 1,
            flags: 0,
            pts_us,
            data: Bytes::from(vec![u8::try_from(pts_us).expect("test PTS"); 1024]),
        })
        .await;
    }
    peer.send(&Message::Ping(0x1234_5678)).await;

    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), peer.recv())
            .await
            .expect("PONG dentro do prazo"),
        Message::Pong(0x1234_5678)
    );
    assert_eq!(
        source.take_keyframe_requests(),
        vec![KeyframeRequest {
            session_id: _welcome.session_id.expect("session id"),
            stream: 1,
        }]
    );
    drop(peer);
    server.shutdown().await;
}

/// Comandos direcionados a uma sessão encerrada falham sem panic.
#[tokio::test]
async fn command_to_closed_session_returns_error() {
    let (mut server, port, _dir) = boot().await;
    let token = pair_token(&server);
    let (peer, welcome) = connect(port, token).await;
    let session_id = welcome.session_id.expect("session id");
    drop(peer);

    tokio::time::timeout(Duration::from_secs(1), async {
        while server.sessions().is_active(&session_id) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("sessão encerra");
    assert_eq!(
        server.request_keyframe(&session_id, 1),
        Err(SessionCommandError::NoActiveSession)
    );
    assert_eq!(
        server.send_set(
            &session_id,
            SetPayload::One(SetItem {
                key: "camera.active".to_owned(),
                value: serde_json::json!("front"),
            }),
        ),
        Err(SessionCommandError::NoActiveSession)
    );
    server.shutdown().await;
}
