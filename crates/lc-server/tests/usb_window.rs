//! Testes da janela de aprovação USB (F1-T07): 15 s, só
//! `127.0.0.1`/`::1`, uso único e token de pareamento ainda obrigatório.
//!
//! Os unitários usam o relógio injetado (`now: u64`); os de integração
//! rodam o handshake real com `auto_approve = false`.

// `common` traz helpers que nem todo teste de integração usa.
#[allow(dead_code)]
mod common;

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use common::TestClient;
use lc_protocol::json::{Reason, Welcome};
use lc_protocol::Message;
use lc_server::config::ServerConfig;
use lc_server::events::{ApprovalOutcome, ServerEvent};
use lc_server::trust::UsbWindow;
use lc_server::{start, ServerHandle};
use tokio::sync::mpsc;

/// Prazo dos `recv` dos clientes de teste.
const TIMEOUT: Duration = Duration::from_secs(3);

/// Peer loopback para os unitários.
fn loopback() -> IpAddr {
    IpAddr::from(Ipv4Addr::LOCALHOST)
}

/// Sobe o servidor (aprovção padrão de 30 s) e devolve (alça, porta, tempdir).
async fn boot() -> (ServerHandle, u16, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = ServerConfig::for_tests(dir.path().to_path_buf(), false);
    let (handle, port) = spawn(cfg).await;
    (handle, port, dir)
}

/// Sobe o servidor e devolve (alça, porta).
async fn spawn(cfg: ServerConfig) -> (ServerHandle, u16) {
    let mut handle = start(cfg).await.expect("start");
    let port = match handle.events().recv().await.expect("evento") {
        ServerEvent::Listening { port } => port,
        other => panic!("esperava Listening, veio {other:?}"),
    };
    (handle, port)
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

/// O primeiro quadro do cliente durante o pareamento é o `PENDING`.
async fn take_pending(c: &mut TestClient) -> lc_protocol::json::Pending {
    match c.recv(TIMEOUT).await.expect("esperava PENDING") {
        Message::Pending(p) => p,
        other => panic!("esperava PENDING, veio {other:?}"),
    }
}

/// O `WELCOME` do cliente.
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

/// Drena eventos por `dur` (devolve o que viu; pode ser menos).
/// Serve para assertivas de **ausência** sem bloquear até `n` eventos.
async fn drain_events(rx: &mut mpsc::Receiver<ServerEvent>, dur: Duration) -> Vec<ServerEvent> {
    let mut out = Vec::new();
    let end = tokio::time::Instant::now() + dur;
    loop {
        let remaining = end.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(ev)) => out.push(ev),
            Ok(None) | Err(_) => break,
        }
    }
    out
}

/// Espera o `ApprovalRequested` e devolve o `request_id`.
async fn take_request(rx: &mut mpsc::Receiver<ServerEvent>) -> u64 {
    let ev = find_event(rx, 8, |e| matches!(e, ServerEvent::ApprovalRequested(_)))
        .await
        .expect("ApprovalRequested");
    match ev {
        ServerEvent::ApprovalRequested(req) => req.request_id,
        _ => unreachable!("find_event devolveu o evento buscado"),
    }
}

/// Espera o `ApprovalResolved` do `request_id` e devolve o desfecho.
async fn take_resolved(rx: &mut mpsc::Receiver<ServerEvent>, request_id: u64) -> ApprovalOutcome {
    let ev = find_event(
        rx,
        8,
        |e| matches!(e, ServerEvent::ApprovalResolved { request_id: rid, .. } if *rid == request_id),
    )
    .await
    .expect("ApprovalResolved");
    match ev {
        ServerEvent::ApprovalResolved { outcome, .. } => outcome,
        _ => unreachable!("find_event devolveu o evento buscado"),
    }
}

/// UT-1.18 — janela de 15 s com limite inclusivo: `+15 s` ainda vale;
/// `+16 s` expirou.
#[test]
fn window_expires_after_15s() {
    let mut w = UsbWindow::new();
    w.open(100);
    assert!(w.is_open(115), "limite inclusivo: +15 s ainda aberta");
    assert!(w.try_consume(loopback(), 115), "consome em +15 s");

    let mut w2 = UsbWindow::new();
    w2.open(100);
    assert!(!w2.try_consume(loopback(), 116), "+16 s está expirada");
    assert!(!w2.is_open(116), "janela fica fechada");
}

/// UT-1.18 — uso único: dois consumos seguidos → `true` e `false`.
#[test]
fn window_is_single_use() {
    let mut w = UsbWindow::new();
    w.open(1000);
    assert!(w.try_consume(loopback(), 1001), "1º consumo");
    assert!(!w.try_consume(loopback(), 1002), "2º consumo falha");
    assert!(!w.is_open(1003), "janela segue fechada após o consumo");
}

/// UT-1.18 — só loopback consome: peer `192.168.0.5` não consome e a
/// janela continua aberta.
#[test]
fn only_loopback_consumes() {
    let mut w = UsbWindow::new();
    w.open(1000);
    let remote = "192.168.0.5".parse::<IpAddr>().expect("ip válido");
    assert!(!w.try_consume(remote, 1001), "nã-loopback não consome");
    assert!(w.is_open(1002), "janela continua aberta");
    assert!(
        w.try_consume(loopback(), 1003),
        "loopback ainda consome dentro da janela"
    );
}

/// AT-1.13 — pareamento de localhost dentro da janela: aceito **sem**
/// `ApprovalRequested` e com `deviceKey` (`ApproveAlways`).
#[tokio::test]
async fn usb_window_auto_approves_localhost() {
    let (mut handle, port, _dir) = boot().await;
    handle.open_usb_window().await;
    assert!(
        find_event(handle.events(), 4, |e| matches!(e, ServerEvent::UsbWindowOpened))
            .await
            .is_some(),
        "UsbWindowOpened"
    );

    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");

    // O 1º quadro é o WELCOME aceito direto (sem PENDING).
    let w = take_welcome(&mut c).await;
    assert!(w.accepted, "pareamento na janela é aceito sem diálogo");
    let key = w.device_key.expect("deviceKey (ApproveAlways)");
    assert_eq!(key.len(), 64, "deviceKey = 256 bits em hex");

    // A janela fecha consumida (uso único).
    assert!(
        find_event(
            handle.events(),
            8,
            |e| matches!(e, ServerEvent::UsbWindowClosed { used: true }),
        )
        .await
        .is_some(),
        "UsbWindowClosed {{ used: true }}"
    );

    // `ApprovalRequested` não chega a ser emitido.
    let evs = drain_events(handle.events(), Duration::from_millis(400)).await;
    assert!(
        evs.iter().all(|e| !matches!(e, ServerEvent::ApprovalRequested(_))),
        "sem ApprovalRequested na janela: {evs:?}"
    );

    handle.shutdown().await;
}

/// AT-1.13 — sem abrir a janela: fluxo normal (`PENDING` +
/// `ApprovalRequested`) e recusa com `not_approved` no timeout.
#[tokio::test]
async fn outside_window_normal_flow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cfg = ServerConfig::for_tests(dir.path().to_path_buf(), false);
    cfg.approval_timeout = Duration::from_millis(300);
    let (mut handle, port) = spawn(cfg).await;

    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    take_pending(&mut c).await;
    let request_id = take_request(handle.events()).await;

    let w = take_welcome(&mut c).await;
    assert!(!w.accepted, "sem janela, o timeout não aprova");
    assert_eq!(w.reason, Some(Reason::NotApproved));

    assert_eq!(
        take_resolved(handle.events(), request_id).await,
        ApprovalOutcome::TimedOut,
    );

    handle.shutdown().await;
    drop(dir);
}

/// ST-5.04 — o 2º pareamento da mesma janela segue o fluxo normal: a
/// janela foi consumida pelo 1º.
#[tokio::test]
async fn second_hello_in_window_not_auto_approved() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cfg = ServerConfig::for_tests(dir.path().to_path_buf(), false);
    cfg.approval_timeout = Duration::from_millis(300);
    let (mut handle, port) = spawn(cfg).await;
    handle.open_usb_window().await;

    // 1º pareamento: consome a janela (aceito sem diálogo).
    let mut c1 = TestClient::connect(port).await.expect("connect");
    c1.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    assert!(take_welcome(&mut c1).await.accepted, "1º consome a janela");

    // 2º pareamento (o token rotacionou no 1º): fluxo normal —
    // PENDING + ApprovalRequested; no timeout, `not_approved`.
    let mut c2 = TestClient::connect(port).await.expect("connect");
    c2.send(&TestClient::hello_pair(&pair_token(&handle), "dev-2"))
        .await
        .expect("send");
    take_pending(&mut c2).await;
    let request_id = take_request(handle.events()).await;

    let w2 = take_welcome(&mut c2).await;
    assert!(!w2.accepted, "2º segue o fluxo normal");
    assert_eq!(w2.reason, Some(Reason::NotApproved), "timeout: not_approved");

    assert_eq!(
        take_resolved(handle.events(), request_id).await,
        ApprovalOutcome::TimedOut,
    );

    handle.shutdown().await;
    drop(dir);
}

/// A janela não dispensa o token: token errado → `bad_token`, e a
/// janela **segue aberta** (só `HELLO` válido consome).
#[tokio::test]
async fn token_still_required() {
    let (mut handle, port, _dir) = boot().await;
    handle.open_usb_window().await;

    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&"f".repeat(32), "dev-1"))
        .await
        .expect("send");
    let w = take_welcome(&mut c).await;
    assert!(!w.accepted);
    assert_eq!(w.reason, Some(Reason::BadToken));
    assert!(c.recv(TIMEOUT).await.is_none(), "socket fecha após a recusa");

    // A janela segue aberta: `HELLO` válido é aceito sem diálogo.
    let mut c2 = TestClient::connect(port).await.expect("connect");
    c2.send(&TestClient::hello_pair(&pair_token(&handle), "dev-2"))
        .await
        .expect("send");
    assert!(
        take_welcome(&mut c2).await.accepted,
        "token inválido não consome a janela"
    );

    handle.shutdown().await;
}
