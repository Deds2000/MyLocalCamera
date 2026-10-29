//! Testes de integração do socket de áudio (F1-T09b): `AUDIO_HELLO`
//! com ticket de uso único, contadores de blocos, isolamento da sessão
//! principal e reabertura/substituição do áudio.
//!
//! O servidor roda com `auto_approve = true`.

#[allow(dead_code)]
mod common;

use std::sync::atomic::Ordering;
use std::time::Duration;

use bytes::Bytes;
use common::TestClient;
use lc_protocol::json::{AudioHello, Reason, Welcome};
use lc_protocol::{encode_frame, msg, Message};
use lc_server::config::ServerConfig;
use lc_server::events::{CloseReason, ServerEvent};
use lc_server::{start, ServerHandle};
use tokio::sync::mpsc;

/// Prazo dos `recv` dos clientes de teste.
const TIMEOUT: Duration = Duration::from_secs(3);

/// `sessionId` falso (32 hex) para tentativas de sessão inexistente.
const FAKE_SID: &str = "0123456789abcdef0123456789abcdef";

async fn boot() -> (ServerHandle, u16, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = ServerConfig::for_tests(dir.path().to_path_buf(), true);
    let mut handle = start(cfg).await.expect("start");
    let port = match handle.events().recv().await.expect("evento") {
        ServerEvent::Listening { port } => port,
        other => panic!("esperava Listening, veio {other:?}"),
    };
    (handle, port, dir)
}

/// `BYE` de teste.
fn bye(reason: &str) -> Message {
    Message::Bye(lc_protocol::json::Bye {
        reason: reason.to_owned(),
    })
}

/// Parea `device_id` (auto-aprovado); devolve (cliente principal,
/// `sessionId`, `audioTicket` do WELCOME).
async fn pair(handle: &ServerHandle, port: u16, device_id: &str) -> (TestClient, String, String) {
    let token = handle
        .trust()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .pair_token()
        .to_owned();
    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&token, device_id))
        .await
        .expect("send");
    match c.recv(TIMEOUT).await.expect("esperava WELCOME") {
        Message::Welcome(w) => {
            assert!(w.accepted, "pareamento deve ser aprovado (auto_approve)");
            (c, w.session_id.expect("sessionId"), w.audio_ticket.expect("audioTicket"))
        }
        other => panic!("esperava WELCOME, veio {other:?}"),
    }
}

/// Abre o socket de áudio com `AUDIO_HELLO` e devolve (cliente, WELCOME).
async fn open_audio(port: u16, session_id: &str, ticket: &str) -> (TestClient, Welcome) {
    let hello = Message::AudioHello(AudioHello {
        session_id: session_id.to_owned(),
        ticket: ticket.to_owned(),
    });
    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&hello).await.expect("send");
    match c.recv(TIMEOUT).await.expect("esperava WELCOME") {
        Message::Welcome(w) => (c, w),
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

/// Pede um novo ticket de áudio no socket principal e devolve o ticket.
async fn request_ticket(c: &mut TestClient) -> String {
    c.send(&Message::AudioTicketRequest).await.expect("send");
    match c.recv(TIMEOUT).await.expect("esperava AUDIO_TICKET") {
        Message::AudioTicket(t) => t.ticket,
        other => panic!("esperava AUDIO_TICKET, veio {other:?}"),
    }
}

// AT-1.09 — 1º áudio aceito com o ticket do WELCOME; 2º uso do mesmo
// ticket → `bad_audio_ticket` (uso único).
#[tokio::test]
async fn audio_ticket_flow_and_reuse_denied() {
    let (mut handle, port, _dir) = boot().await;
    let (_primary, sid, ticket) = pair(&handle, port, "dev-1").await;

    let (_a1, w1) = open_audio(port, &sid, &ticket).await;
    assert!(w1.accepted, "1º uso deve ser aceito");
    assert_eq!(w1.session_id.as_deref(), Some(sid.as_str()));

    let (mut a2, w2) = open_audio(port, &sid, &ticket).await;
    assert!(!w2.accepted, "2º uso do ticket deve ser recusado");
    assert_eq!(w2.reason, Some(Reason::BadAudioTicket));
    assert!(a2.recv(TIMEOUT).await.is_none(), "2º socket deve fechar");

    handle.shutdown().await;
}

// 50 blocos de 960 bytes → `audio_blocks == 50` e `audio_bytes == 48_000`.
#[tokio::test]
async fn audio_blocks_are_counted() {
    let (mut handle, port, _dir) = boot().await;
    let (_primary, sid, ticket) = pair(&handle, port, "dev-1").await;

    let (mut a, w) = open_audio(port, &sid, &ticket).await;
    assert!(w.accepted);
    // `encode_frame` direto: o helper `send` fixa `pts_us = 0`.
    for i in 0..50u64 {
        let frame = encode_frame(msg::AUDIO, 0, 3, i * 1_000, &[0u8; 960]).expect("encode");
        a.send_raw(&frame).await.expect("send");
    }
    a.send(&bye("done")).await.expect("send");

    // `AudioClosed` é emitido só depois de processar os blocos.
    assert!(
        find_event(handle.events(), 8, |e| matches!(e, ServerEvent::AudioClosed { .. }))
            .await
            .is_some(),
        "esperava AudioClosed"
    );

    let c = handle.sessions().counters(&sid).expect("sessão ativa");
    assert_eq!(c.audio_blocks.load(Ordering::Relaxed), 50);
    assert_eq!(c.audio_bytes.load(Ordering::Relaxed), 48_000);
    assert_eq!(c.last_audio_pts.load(Ordering::Relaxed), 49_000);

    handle.shutdown().await;
}

// FT-1.10 — `AUDIO_HELLO` com `sessionId` inexistente: `bad_audio_ticket`.
#[tokio::test]
async fn audio_hello_without_session_denied() {
    let (mut handle, port, _dir) = boot().await;
    let (mut a, w) = open_audio(port, FAKE_SID, &"f".repeat(32)).await;
    assert!(!w.accepted);
    assert_eq!(w.reason, Some(Reason::BadAudioTicket));
    assert!(a.recv(TIMEOUT).await.is_none(), "socket deve fechar");

    handle.shutdown().await;
}

// Ticket de outra sessão (resgatado com `sessionId` diferente do dono):
// `bad_audio_ticket`, e o ticket **não** é consumido.
#[tokio::test]
async fn ticket_of_other_session_denied() {
    let (mut handle, port, _dir) = boot().await;
    let (_primary, sid, ticket) = pair(&handle, port, "dev-1").await;
    assert_ne!(sid, FAKE_SID, "sessionId real não pode ser o falso");

    let (mut a, w) = open_audio(port, FAKE_SID, &ticket).await;
    assert!(!w.accepted);
    assert_eq!(w.reason, Some(Reason::BadAudioTicket));
    assert!(a.recv(TIMEOUT).await.is_none(), "socket deve fechar");

    // O ticket segue válido para a sessão que o emitiu.
    let (_a2, w2) = open_audio(port, &sid, &ticket).await;
    assert!(w2.accepted, "ticket deve seguir válido na sessão de origem");

    handle.shutdown().await;
}

// IT-1.07 — áudio cai: principal intacta (PING/PONG); novo ticket via
// `AUDIO_TICKET_REQUEST`; áudio reabre.
#[tokio::test]
async fn audio_drop_keeps_primary_and_reopens() {
    let (mut handle, port, _dir) = boot().await;
    let (mut primary, sid, ticket) = pair(&handle, port, "dev-1").await;

    let (mut a1, w1) = open_audio(port, &sid, &ticket).await;
    assert!(w1.accepted);
    a1.send(&bye("drop")).await.expect("send");
    assert!(a1.recv(TIMEOUT).await.is_none(), "áudio fecha após BYE");

    // Principal intacta.
    primary.send(&Message::Ping(7)).await.expect("send");
    assert_eq!(primary.recv(TIMEOUT).await, Some(Message::Pong(7)), "principal viva");

    // Novo ticket e reabertura.
    let ticket2 = request_ticket(&mut primary).await;
    assert_ne!(ticket2, ticket, "novo ticket difere do anterior");
    let (_a2, w2) = open_audio(port, &sid, &ticket2).await;
    assert!(w2.accepted, "áudio deve reabrir");

    handle.shutdown().await;
}

// Encerrada a principal, o áudio fecha. Motivo: `PeerClosed` (a sessão
// que o donava deixa de ser a ativa — documentado em `audio/conn.rs`).
#[tokio::test]
async fn primary_close_closes_audio() {
    let (mut handle, port, _dir) = boot().await;
    let (mut primary, sid, ticket) = pair(&handle, port, "dev-1").await;

    let (mut a, w) = open_audio(port, &sid, &ticket).await;
    assert!(w.accepted);

    primary.send(&bye("done")).await.expect("send");
    assert!(primary.recv(TIMEOUT).await.is_none(), "principal fecha");
    assert!(a.recv(TIMEOUT).await.is_none(), "áudio deve fechar com a sessão");

    assert!(
        find_event(handle.events(), 12, |e| matches!(
            e,
            ServerEvent::AudioClosed { reason, session_id, .. }
                if *reason == CloseReason::PeerClosed && *session_id == sid
        ))
        .await
        .is_some(),
        "esperava AudioClosed com PeerClosed"
    );

    handle.shutdown().await;
}

// Bloco de 16 385 bytes (acima do máximo de 16 KiB): áudio fecha com
// `ProtocolError`; a principal segue viva.
#[tokio::test]
async fn audio_oversize_closes_only_audio() {
    let (mut handle, port, _dir) = boot().await;
    let (mut primary, sid, ticket) = pair(&handle, port, "dev-1").await;

    let (mut a, w) = open_audio(port, &sid, &ticket).await;
    assert!(w.accepted);
    a.send(&Message::Audio {
        pts_us: 0,
        data: Bytes::from(vec![0u8; 16_385]),
    })
    .await
    .expect("send");
    assert!(a.recv(TIMEOUT).await.is_none(), "áudio fecha com ProtocolError");

    assert!(
        find_event(handle.events(), 10, |e| matches!(
            e,
            ServerEvent::AudioClosed { reason, .. }
                if *reason == CloseReason::ProtocolError
        ))
        .await
        .is_some(),
        "esperava AudioClosed com ProtocolError"
    );

    // Principal segue viva.
    primary.send(&Message::Ping(3)).await.expect("send");
    assert_eq!(primary.recv(TIMEOUT).await, Some(Message::Pong(3)), "principal viva");

    handle.shutdown().await;
}

// 2º áudio (ticket via request) substitui o 1º: o 1º fecha com
// `Replaced` e o 2º segue ativo.
#[tokio::test]
async fn new_audio_replaces_old() {
    let (mut handle, port, _dir) = boot().await;
    let (mut primary, sid, ticket) = pair(&handle, port, "dev-1").await;

    let (mut a1, w1) = open_audio(port, &sid, &ticket).await;
    assert!(w1.accepted);

    let ticket2 = request_ticket(&mut primary).await;
    let (mut a2, w2) = open_audio(port, &sid, &ticket2).await;
    assert!(w2.accepted, "2º áudio deve ser aceito");

    assert!(a1.recv(TIMEOUT).await.is_none(), "1º áudio deve fechar");
    assert!(
        find_event(handle.events(), 14, |e| matches!(
            e,
            ServerEvent::AudioClosed { reason, .. }
                if *reason == CloseReason::Replaced
        ))
        .await
        .is_some(),
        "esperava AudioClosed com Replaced"
    );

    a2.send(&Message::Ping(5)).await.expect("send");
    assert_eq!(a2.recv(TIMEOUT).await, Some(Message::Pong(5)), "2º áudio ativo");

    handle.shutdown().await;
}

// PING no socket de áudio: PONG com o mesmo valor.
#[tokio::test]
async fn audio_ping_gets_pong() {
    let (mut handle, port, _dir) = boot().await;
    let (_primary, sid, ticket) = pair(&handle, port, "dev-1").await;

    let (mut a, w) = open_audio(port, &sid, &ticket).await;
    assert!(w.accepted);
    a.send(&Message::Ping(42)).await.expect("send");
    assert_eq!(a.recv(TIMEOUT).await, Some(Message::Pong(42)), "PONG com o mesmo u64");

    handle.shutdown().await;
}
