//! Testes de integração do handshake (F1-T04b): `HELLO` → `WELCOME`,
//! autenticação `pair`/`device`, negociação de versão, sessão única e
//! proteção contra mensagens fora de ordem.
//!
//! O servidor roda com `auto_approve = true`, salvo indicação.

mod common;

use std::time::Duration;

use common::TestClient;
use lc_protocol::json::{AudioHello, ProtocolVersion, Reason, SetItem, SetPayload, Welcome};
use lc_protocol::{encode_frame, msg, Message};
use lc_server::config::ServerConfig;
use lc_server::events::{CloseReason, ServerEvent};
use lc_server::version::negotiate;
use lc_server::{start, ServerHandle};
use tokio::sync::mpsc;

/// Prazo dos `recv` dos clientes de teste.
const TIMEOUT: Duration = Duration::from_secs(3);

/// Sobe o servidor com `auto_approve` e devolve (alça, porta, tempdir).
async fn boot(auto_approve: bool) -> (ServerHandle, u16, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = ServerConfig::for_tests(dir.path().to_path_buf(), auto_approve);
    let mut handle = start(cfg).await.expect("start");
    let port = match handle.events().recv().await.expect("evento") {
        ServerEvent::Listening { port } => port,
        other => panic!("esperava Listening, veio {other:?}"),
    };
    (handle, port, dir)
}

/// Token de pareamento atual na loja de confiança do servidor.
fn pair_token(handle: &ServerHandle) -> String {
    handle
        .trust()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .pair_token()
        .to_owned()
}

/// Espera o `WELCOME` do cliente.
async fn take_welcome(c: &mut TestClient) -> Welcome {
    match c.recv(TIMEOUT).await.expect("esperava WELCOME") {
        Message::Welcome(w) => w,
        other => panic!("esperava WELCOME, veio {other:?}"),
    }
}

/// Varre até `n` eventos procurando um que satisfaça `pred`.
async fn find_event(
    rx: &mut mpsc::Receiver<ServerEvent>,
    n: usize,
    mut pred: impl FnMut(&ServerEvent) -> bool,
) -> Option<ServerEvent> {
    for _ in 0..n {
        let ev = tokio::time::timeout(TIMEOUT, rx.recv())
            .await
            .ok()
            .flatten()
            .expect("canal aberto");
        if pred(&ev) {
            return Some(ev);
        }
    }
    None
}

// AT-1.02 — pair com token correto: WELCOME aceito com `deviceKey`
// (64 hex) e `audioTicket`; o token de pareamento rotaciona.
#[tokio::test]
async fn pair_ok_returns_device_key_and_rotates_token() {
    let (mut handle, port, _dir) = boot(true).await;
    let before = pair_token(&handle);

    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&before, "dev-1")).await.expect("send");
    let w = take_welcome(&mut c).await;

    assert!(w.accepted, "pair deve ser aprovado (auto_approve)");
    assert_eq!(w.protocol, Some(ProtocolVersion { major: 1, minor: 0 }));
    assert!(w.reason.is_none());
    let sid = w.session_id.expect("sessionId");
    assert_eq!(sid.len(), 32, "sessionId = 16 bytes em hex");
    let ticket = w.audio_ticket.expect("audioTicket");
    assert_eq!(ticket.len(), 32, "audioTicket = 128 bits em hex");
    let key = w.device_key.expect("deviceKey (permitir sempre)");
    assert_eq!(key.len(), 64, "deviceKey = 256 bits em hex");
    assert!(w.pc.is_some(), "WELCOME deve trazer pc");

    // Evento `SessionStarted` com a mesma sessão.
    assert!(
        find_event(handle.events(), 4, |e| matches!(e, ServerEvent::ConnectionOpened { .. }))
            .await
            .is_some(),
        "ConnectionOpened"
    );
    match find_event(handle.events(), 8, |e| matches!(e, ServerEvent::SessionStarted { .. }))
        .await
    {
        Some(ServerEvent::SessionStarted {
            session_id,
            device_id,
            ..
        }) => {
            assert_eq!(session_id, sid);
            assert_eq!(device_id, "dev-1");
        }
        other => panic!("esperava SessionStarted, veio {other:?}"),
    }

    let after = pair_token(&handle);
    assert_ne!(before, after, "token de pareamento deve rotacionar");

    let devs = handle.trust().lock().unwrap().list();
    assert!(
        devs.iter().any(|d| d.device_id == "dev-1"),
        "dispositivo deve estar cadastrado na confiança"
    );

    handle.shutdown().await;
}

// AT-1.03 — token errado: `WELCOME{bad_token}` e socket fechado.
#[tokio::test]
async fn bad_token_denied() {
    let (mut handle, port, _dir) = boot(true).await;
    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&"f".repeat(32), "dev-1"))
        .await
        .expect("send");

    let w = take_welcome(&mut c).await;
    assert!(!w.accepted);
    assert_eq!(w.reason, Some(Reason::BadToken));
    assert!(w.session_id.is_none());
    assert!(w.device_key.is_none());
    assert!(w.audio_ticket.is_none());

    assert!(c.recv(TIMEOUT).await.is_none(), "socket deve fechar após a recusa");

    // O servidor segue aceitando.
    let mut c2 = TestClient::connect(port).await.expect("connect");
    c2.send(&TestClient::hello_pair(&pair_token(&handle), "dev-2"))
        .await
        .expect("send");
    assert!(take_welcome(&mut c2).await.accepted, "servidor segue aceitando");

    handle.shutdown().await;
}

// AT-1.04 — pareia e reconecta com `device`: aceito, **sem** `deviceKey`.
#[tokio::test]
async fn device_reconnect_without_key_in_welcome() {
    let (mut handle, port, _dir) = boot(true).await;

    let mut c1 = TestClient::connect(port).await.expect("connect");
    c1.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    let key = take_welcome(&mut c1).await.device_key.expect("deviceKey");
    drop(c1);

    let mut c2 = TestClient::connect(port).await.expect("connect");
    c2.send(&TestClient::hello_device("dev-1", &key)).await.expect("send");
    let w = take_welcome(&mut c2).await;
    assert!(w.accepted, "auth `device` deve ser aceita");
    assert!(w.device_key.is_none(), "reconexão não recebe deviceKey");
    assert!(w.session_id.is_some());

    // `last_seen`/`last_ip` atualizados (touch).
    let devs = handle.trust().lock().unwrap().list();
    let dev = devs
        .iter()
        .find(|d| d.device_id == "dev-1")
        .expect("dev-1 cadastrado");
    assert!(!dev.last_ip.is_empty(), "last_ip deve ser atualizado");

    handle.shutdown().await;
}

// AT-1.05 — dispositivo revogado reconecta: `revoked`.
#[tokio::test]
async fn revoked_device_denied() {
    let (mut handle, port, _dir) = boot(true).await;

    let mut c1 = TestClient::connect(port).await.expect("connect");
    c1.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    let key = take_welcome(&mut c1).await.device_key.expect("deviceKey");
    drop(c1);

    handle
        .trust()
        .lock()
        .unwrap()
        .revoke("dev-1")
        .expect("revoke");

    let mut c2 = TestClient::connect(port).await.expect("connect");
    c2.send(&TestClient::hello_device("dev-1", &key)).await.expect("send");
    let w = take_welcome(&mut c2).await;
    assert!(!w.accepted);
    assert_eq!(w.reason, Some(Reason::Revoked));
    assert!(c2.recv(TIMEOUT).await.is_none(), "socket deve fechar");

    handle.shutdown().await;
}

// Chave de dispositivo errada: `bad_token`.
#[tokio::test]
async fn device_bad_key_is_bad_token() {
    let (mut handle, port, _dir) = boot(true).await;

    let mut c1 = TestClient::connect(port).await.expect("connect");
    c1.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    let _key = take_welcome(&mut c1).await.device_key.expect("deviceKey");
    drop(c1);

    let mut c2 = TestClient::connect(port).await.expect("connect");
    c2.send(&TestClient::hello_device("dev-1", &"0".repeat(64)))
        .await
        .expect("send");
    let w = take_welcome(&mut c2).await;
    assert!(!w.accepted);
    assert_eq!(w.reason, Some(Reason::BadToken));
    assert!(c2.recv(TIMEOUT).await.is_none(), "socket deve fechar");

    handle.shutdown().await;
}

// UT-1.23 — matriz de negociação (servidor é o `SERVER_PROTOCOL` = 1.0;
// a linha "1.0×1.3" usa a mesma regra: `major` do cliente × `major` do
// servidor — ambos 1).
#[test]
fn negotiate_matrix() {
    let v = |major: u16, minor: u16| ProtocolVersion { major, minor };
    assert!(negotiate(&v(1, 0)).is_ok(), "1.0×1.0");
    assert!(negotiate(&v(1, 3)).is_ok(), "1.3×1.0 (minor tolerado)");
    assert!(negotiate(&v(1, 0)).is_ok(), "1.0×1.3 (major igual)");
    assert_eq!(negotiate(&v(2, 0)), Err(v(1, 0)), "1.0×2.0 → mismatch");
    assert_eq!(negotiate(&v(0, 9)), Err(v(1, 0)), "0.9×1.0 → mismatch");
}

// AT-1.07 — `major` 2: `version_mismatch` + `minSupported {1,0}`.
#[tokio::test]
async fn version_mismatch_has_min_supported() {
    let (mut handle, port, _dir) = boot(true).await;
    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair_v(&pair_token(&handle), "dev-1", 2, 0))
        .await
        .expect("send");

    let w = take_welcome(&mut c).await;
    assert!(!w.accepted);
    assert_eq!(w.reason, Some(Reason::VersionMismatch));
    assert_eq!(
        w.min_supported,
        Some(ProtocolVersion { major: 1, minor: 0 }),
        "minSupported deve ser {{1,0}}"
    );
    assert!(c.recv(TIMEOUT).await.is_none(), "socket deve fechar");

    handle.shutdown().await;
}

// AT-1.08 — 2º cliente de **outro** dispositivo: `busy`; o 1º segue ativo.
#[tokio::test]
async fn busy_for_other_device() {
    let (mut handle, port, _dir) = boot(true).await;

    let mut c1 = TestClient::connect(port).await.expect("connect");
    c1.send(&TestClient::hello_pair(&pair_token(&handle), "dev-A"))
        .await
        .expect("send");
    assert!(take_welcome(&mut c1).await.accepted);

    // O token rotacionou no 1º pareamento.
    let token2 = pair_token(&handle);
    let mut c2 = TestClient::connect(port).await.expect("connect");
    c2.send(&TestClient::hello_pair(&token2, "dev-B")).await.expect("send");

    let w = take_welcome(&mut c2).await;
    assert!(!w.accepted);
    assert_eq!(w.reason, Some(Reason::Busy));
    assert!(c2.recv(TIMEOUT).await.is_none(), "socket deve fechar");

    // A primeira sessão segue ativa.
    c1.send(&Message::Ping(7)).await.expect("send");
    assert_eq!(c1.recv(TIMEOUT).await, Some(Message::Pong(7)), "c1 segue ativo");

    handle.shutdown().await;
}

// IT-1.05 — mesmo dispositivo conecta 2×: o 1º recebe `BYE{"replaced"}`
// e fecha; o 2º fica ativo.
#[tokio::test]
async fn same_device_replaces_session() {
    let (mut handle, port, _dir) = boot(true).await;

    let mut c1 = TestClient::connect(port).await.expect("connect");
    c1.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    let w1 = take_welcome(&mut c1).await;
    let key = w1.device_key.expect("deviceKey");
    let old_sid = w1.session_id.expect("sessionId");

    let mut c2 = TestClient::connect(port).await.expect("connect");
    c2.send(&TestClient::hello_device("dev-1", &key)).await.expect("send");
    assert!(take_welcome(&mut c2).await.accepted, "2ª conexão aceita");

    // O 1º recebe BYE e o socket fecha.
    match c1.recv(TIMEOUT).await.expect("esperava BYE") {
        Message::Bye(bye) => assert_eq!(bye.reason, "replaced"),
        other => panic!("esperava BYE(replaced), veio {other:?}"),
    }
    assert!(c1.recv(TIMEOUT).await.is_none(), "1º socket deve fechar");

    // O 2º está ativo.
    c2.send(&Message::Ping(9)).await.expect("send");
    assert_eq!(c2.recv(TIMEOUT).await, Some(Message::Pong(9)));

    // A sessão antiga encerrou com `Replaced`.
    match find_event(handle.events(), 16, |e| matches!(
        e,
        ServerEvent::SessionEnded { session_id, .. } if *session_id == old_sid
    ))
    .await
    {
        Some(ServerEvent::SessionEnded { reason, .. }) => {
            assert_eq!(reason, CloseReason::Replaced)
        }
        other => panic!("esperava SessionEnded da sessão antiga: {other:?}"),
    }

    handle.shutdown().await;
}

// AT-1.14 — JSON inválido no HELLO: fechado (`ProtocolError`), sem
// resposta; o servidor segue aceitando.
#[tokio::test]
async fn bad_json_hello_is_protocol_error() {
    let (mut handle, port, _dir) = boot(true).await;
    let mut c = TestClient::connect(port).await.expect("connect");

    let bad = encode_frame(msg::HELLO, 0, 0, 0, b"{not json").expect("encode");
    c.send_raw(&bad).await.expect("send");

    assert!(c.recv(TIMEOUT).await.is_none(), "fecha calado, sem resposta");

    // O servidor segue aceitando.
    let mut c2 = TestClient::connect(port).await.expect("connect");
    c2.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    assert!(take_welcome(&mut c2).await.accepted);

    handle.shutdown().await;
}

// FT-1.04 — HELLO de 65 537 bytes: fechado sem alocar tudo.
#[tokio::test]
async fn huge_hello_rejected() {
    let (mut handle, port, _dir) = boot(true).await;
    let mut c = TestClient::connect(port).await.expect("connect");

    let payload = vec![b'a'; 65_537];
    let frame = encode_frame(msg::HELLO, 0, 0, 0, &payload).expect("encode");
    c.send_raw(&frame).await.expect("send");

    assert!(c.recv(TIMEOUT).await.is_none(), "fecha sem resposta");

    // O servidor segue aceitando.
    let mut c2 = TestClient::connect(port).await.expect("connect");
    c2.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    assert!(take_welcome(&mut c2).await.accepted);

    handle.shutdown().await;
}

// ST-1.04 — PING/SET/VIDEO como 1º quadro: fechado, sem resposta.
#[tokio::test]
async fn message_before_welcome_closes() {
    let (mut handle, port, _dir) = boot(true).await;

    let first_frames = [
        Message::Ping(1),
        Message::Set(SetPayload::One(SetItem {
            key: "camera.mode".to_owned(),
            value: serde_json::json!("single"),
        })),
        Message::Video {
            stream: 1,
            flags: 1,
            pts_us: 0,
            data: bytes::Bytes::from(vec![0, 1, 2, 3]),
        },
    ];
    for first in first_frames {
        let mut c = TestClient::connect(port).await.expect("connect");
        c.send(&first).await.expect("send");
        assert!(
            c.recv(TIMEOUT).await.is_none(),
            "{first:?} como 1º quadro deve fechar sem resposta"
        );
    }

    handle.shutdown().await;
}

// FT-1.10 — `AUDIO_HELLO` como 1º quadro: `bad_audio_ticket`.
#[tokio::test]
async fn audio_hello_before_welcome_denied() {
    let (mut handle, port, _dir) = boot(true).await;
    let mut c = TestClient::connect(port).await.expect("connect");

    c.send(&Message::AudioHello(AudioHello {
        session_id: "s".to_owned(),
        ticket: "t".to_owned(),
    }))
    .await
    .expect("send");

    let w = take_welcome(&mut c).await;
    assert!(!w.accepted);
    assert_eq!(w.reason, Some(Reason::BadAudioTicket));
    assert!(c.recv(TIMEOUT).await.is_none(), "socket deve fechar");

    handle.shutdown().await;
}

// `HELLO` de novo numa conexão autenticada: `ProtocolError`.
#[tokio::test]
async fn second_hello_is_protocol_error() {
    let (mut handle, port, _dir) = boot(true).await;
    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    assert!(take_welcome(&mut c).await.accepted);

    c.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    assert!(c.recv(TIMEOUT).await.is_none(), "2º HELLO fecha a conexão");

    handle.shutdown().await;
}

// PING em sessão ativa: PONG com o mesmo `u64`.
#[tokio::test]
async fn ping_gets_pong() {
    let (mut handle, port, _dir) = boot(true).await;
    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    assert!(take_welcome(&mut c).await.accepted);

    c.send(&Message::Ping(42)).await.expect("send");
    assert_eq!(
        c.recv(TIMEOUT).await,
        Some(Message::Pong(42)),
        "PONG deve carregar o mesmo u64"
    );

    handle.shutdown().await;
}
