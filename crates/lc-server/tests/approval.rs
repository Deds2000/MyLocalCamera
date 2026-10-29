//! Testes de integração do fluxo de aprovação (F1-T06): `PENDING`,
//! decisão da UI (`approve_always`/`approve_once`/`reject`), timeout,
//! cancelamento por desconexão e um pedido pendente por vez.
//!
//! O servidor roda com `auto_approve = false`, para todo pareamento
//! passar pelo pedido real (`ApprovalRequested` + `decide`).

// `common` traz helpers que nem todo teste de integração usa.
#[allow(dead_code)]
mod common;

use std::time::Duration;

use common::TestClient;
use lc_protocol::json::{Pending, Reason, Welcome};
use lc_protocol::Message;
use lc_server::config::ServerConfig;
use lc_server::events::{ApprovalOutcome, Decision, ServerEvent};
use lc_server::trust::effective_auto_approve;
use lc_server::{start, ServerHandle};
use tokio::sync::mpsc;

/// Prazo dos `recv` dos clientes de teste.
const TIMEOUT: Duration = Duration::from_secs(3);

/// Sobe o servidor e devolve (alça, porta).
async fn spawn(cfg: ServerConfig) -> (ServerHandle, u16) {
    let mut handle = start(cfg).await.expect("start");
    let port = match handle.events().recv().await.expect("evento") {
        ServerEvent::Listening { port } => port,
        other => panic!("esperava Listening, veio {other:?}"),
    };
    (handle, port)
}

/// Sobe o servidor padrão de testes com `auto_approve` dado.
async fn boot(auto_approve: bool) -> (ServerHandle, u16, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = ServerConfig::for_tests(dir.path().to_path_buf(), auto_approve);
    let (handle, port) = spawn(cfg).await;
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

/// O primeiro quadro do cliente durante o pareamento é o `PENDING`.
async fn take_pending(c: &mut TestClient) -> Pending {
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
    let ev = find_event(rx, 8, |e| matches!(e, ServerEvent::ApprovalResolved { request_id: rid, .. } if *rid == request_id))
        .await
        .expect("ApprovalResolved");
    match ev {
        ServerEvent::ApprovalResolved { outcome, .. } => outcome,
        _ => unreachable!("find_event devolveu o evento buscado"),
    }
}

// UT-1.16 — `ApproveAlways`: aceito com `deviceKey`; dispositivo fica
// em `trust.list()`; desfecho `Approved(ApproveAlways)`.
#[tokio::test]
async fn approve_always_persists() {
    let (mut handle, port, _dir) = boot(false).await;
    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    take_pending(&mut c).await;
    let request_id = take_request(handle.events()).await;
    assert!(
        handle.decide(request_id, Decision::ApproveAlways),
        "decide no pedido pendente"
    );

    let w = take_welcome(&mut c).await;
    assert!(w.accepted, "ApproveAlways aceita");
    let key = w.device_key.expect("deviceKey (permitir sempre)");
    assert_eq!(key.len(), 64, "deviceKey = 256 bits em hex");

    let devs = handle.trust().lock().unwrap().list();
    assert!(
        devs.iter().any(|d| d.device_id == "dev-1"),
        "dispositivo em trust.list()"
    );

    assert_eq!(
        take_resolved(handle.events(), request_id).await,
        ApprovalOutcome::Approved(Decision::ApproveAlways),
    );

    handle.shutdown().await;
}

// UT-1.16 — `ApproveOnce`: aceito **sem** `deviceKey`; nada persiste.
#[tokio::test]
async fn approve_once_does_not_persist() {
    let (mut handle, port, _dir) = boot(false).await;
    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    take_pending(&mut c).await;
    let request_id = take_request(handle.events()).await;
    assert!(
        handle.decide(request_id, Decision::ApproveOnce),
        "decide no pedido pendente"
    );

    let w = take_welcome(&mut c).await;
    assert!(w.accepted, "ApproveOnce aceita");
    assert!(w.device_key.is_none(), "sem deviceKey em 'só desta vez'");

    assert!(
        handle.trust().lock().unwrap().list().is_empty(),
        "lista de confiáveis segue vazia"
    );

    assert_eq!(
        take_resolved(handle.events(), request_id).await,
        ApprovalOutcome::Approved(Decision::ApproveOnce),
    );

    handle.shutdown().await;
}

// UT-1.16 — `Reject`: `not_approved` e socket fechado.
#[tokio::test]
async fn reject_denies() {
    let (mut handle, port, _dir) = boot(false).await;
    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    take_pending(&mut c).await;
    let request_id = take_request(handle.events()).await;
    assert!(
        handle.decide(request_id, Decision::Reject),
        "decide no pedido pendente"
    );

    let w = take_welcome(&mut c).await;
    assert!(!w.accepted);
    assert_eq!(w.reason, Some(Reason::NotApproved));
    assert!(
        c.recv(TIMEOUT).await.is_none(),
        "socket fecha após a recusa"
    );

    assert_eq!(
        take_resolved(handle.events(), request_id).await,
        ApprovalOutcome::Rejected,
    );
    assert!(
        handle.trust().lock().unwrap().list().is_empty(),
        "nada foi cadastrado"
    );

    handle.shutdown().await;
}

// AT-1.06 — sem decisão, o timeout (300 ms) recusa com `not_approved`
// e o desfecho é `TimedOut`.
#[tokio::test]
async fn approval_timeout_denies() {
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
    assert!(!w.accepted, "timeout não aprova");
    assert_eq!(w.reason, Some(Reason::NotApproved));

    assert_eq!(
        take_resolved(handle.events(), request_id).await,
        ApprovalOutcome::TimedOut,
    );
    assert!(handle.trust().lock().unwrap().list().is_empty());

    handle.shutdown().await;
    drop(dir);
}

// FT-1.08 — celular fecha o socket durante o pedido: `Cancelled` e o
// pedido é liberado (um novo pareamento funciona).
#[tokio::test]
async fn phone_disconnect_cancels_pending() {
    let (mut handle, port, _dir) = boot(false).await;

    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    take_pending(&mut c).await;
    let request_id = take_request(handle.events()).await;
    drop(c); // celular desconectou

    assert_eq!(
        take_resolved(handle.events(), request_id).await,
        ApprovalOutcome::Cancelled,
        "desconexão cancela o pedido"
    );

    // O pedido foi liberado: um novo pareamento funciona normalmente.
    let mut c2 = TestClient::connect(port).await.expect("connect");
    c2.send(&TestClient::hello_pair(&pair_token(&handle), "dev-2"))
        .await
        .expect("send");
    take_pending(&mut c2).await;
    let id2 = take_request(handle.events()).await;
    assert_ne!(id2, request_id);
    assert!(handle.decide(id2, Decision::ApproveOnce));
    assert!(
        take_welcome(&mut c2).await.accepted,
        "novo pareamento aceito"
    );

    handle.shutdown().await;
}

// FT-1.07 (parcial; a tempestade completa é do F1-T15) — 2º cliente
// pareia com pedido pendente → `busy`; o 1º segue decidível.
#[tokio::test]
async fn second_pair_while_pending_is_busy() {
    let (mut handle, port, _dir) = boot(false).await;

    let mut c1 = TestClient::connect(port).await.expect("connect");
    let token = pair_token(&handle);
    c1.send(&TestClient::hello_pair(&token, "dev-1"))
        .await
        .expect("send");
    take_pending(&mut c1).await;
    let request_id = take_request(handle.events()).await;

    let mut c2 = TestClient::connect(port).await.expect("connect");
    c2.send(&TestClient::hello_pair(&token, "dev-2"))
        .await
        .expect("send");
    // c2 recebe WELCOME direto (sem PENDING: o pedido não abriu)
    let w2 = take_welcome(&mut c2).await;
    assert!(!w2.accepted, "2º pareamento é recusado");
    assert_eq!(w2.reason, Some(Reason::Busy));
    assert!(c2.recv(TIMEOUT).await.is_none(), "socket do 2º fecha");

    // O 1º pedido segue válido e decidível.
    assert!(handle.decide(request_id, Decision::ApproveOnce));
    assert!(
        take_welcome(&mut c1).await.accepted,
        "1º pedido segue ativo"
    );

    handle.shutdown().await;
}

// ST-3.06 — `decide(9999, …)` em `request_id` desconhecido é ignorado:
// `false` e nada muda; o fluxo real segue funcionando.
#[tokio::test]
async fn decide_unknown_request_ignored() {
    let (mut handle, port, _dir) = boot(false).await;

    assert!(
        !handle.decide(9999, Decision::ApproveAlways),
        "id desconhecido devolve false"
    );
    assert!(handle.trust().lock().unwrap().list().is_empty());

    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");
    take_pending(&mut c).await;
    let request_id = take_request(handle.events()).await;
    assert!(
        handle.decide(request_id, Decision::ApproveOnce),
        "fluxo real ok"
    );
    assert!(take_welcome(&mut c).await.accepted);
    assert!(
        !handle.decide(9999, Decision::ApproveAlways),
        "id desconhecido segue ignorado"
    );

    handle.shutdown().await;
}

// ST-1.06 — `auto_approve` só vale em build de debug.
#[test]
fn auto_approve_ignored_in_release() {
    assert!(!effective_auto_approve(true, false), "release: ignorado");
    assert!(effective_auto_approve(true, true), "debug: vale");
    assert!(!effective_auto_approve(false, true), "flag desligada: nada");
}

// O cliente recebe `PENDING{timeoutS:30}` durante o pedido (padrão de
// 30 s).
#[tokio::test]
async fn pending_message_sent() {
    let (mut handle, port, _dir) = boot(false).await;
    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&pair_token(&handle), "dev-1"))
        .await
        .expect("send");

    let p = take_pending(&mut c).await;
    assert_eq!(p.timeout_s, 30, "PENDING com timeoutS 30 (padrão)");

    let request_id = take_request(handle.events()).await;
    assert!(handle.decide(request_id, Decision::ApproveOnce));
    assert!(take_welcome(&mut c).await.accepted);

    handle.shutdown().await;
}
