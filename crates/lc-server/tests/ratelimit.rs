//! Limitadores (F1-T15): falhas de autenticação por IP e taxa de novas
//! conexões. Unitários com relógio virtual; integração com servidor real
//! (valores pequenos de bloqueio para o teste seguir rápido).

// `common` traz helpers que nem todo teste de integração usa.
#[allow(dead_code)]
mod common;

use std::net::IpAddr;
use std::time::Duration;

use common::TestClient;
use lc_protocol::json::{Reason, Welcome};
use lc_protocol::Message;
use lc_server::config::ServerConfig;
use lc_server::events::{CloseReason, ServerEvent};
use lc_server::trust::ratelimit::{AuthLimiter, ConnRateLimiter};
use lc_server::{start, ServerHandle};

/// Prazo dos `recv` dos clientes de teste.
const TIMEOUT: Duration = Duration::from_secs(3);

fn ip(s: &str) -> IpAddr {
    s.parse().expect("ip válido")
}

/// Sobe o servidor (bloqueio de auth em segundos) e devolve (alça, porta, tempdir).
async fn boot(auto_approve: bool, auth_block: u64) -> (ServerHandle, u16, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cfg = ServerConfig::for_tests(dir.path().to_path_buf(), auto_approve);
    cfg.auth_fail_max = 5;
    cfg.auth_fail_window = 60;
    cfg.auth_block = auth_block;
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

/// UT-1.24 — 5 falhas em 60 s: bloqueio de 5 min; expira (relógio virtual).
#[test]
fn blocks_after_max_failures() {
    let a = ip("10.0.0.1");
    let mut l = AuthLimiter::new(5, 60, 300);
    for i in 0..4u64 {
        assert!(!l.record_failure(a, 1000 + i), "sem bloqueio antes das 5 falhas");
    }
    assert!(l.record_failure(a, 1004), "a 5ª falha entra em bloqueio");
    assert!(l.is_blocked(a, 1010), "bloqueado após as 5 falhas");
    assert!(l.is_blocked(a, 1303), "bloqueio dura 5 min (a partir da última falha)");
    assert!(!l.is_blocked(a, 1304), "bloqueio expira");
}

/// UT-1.24 — 4 falhas, 1 sucesso, 4 falhas: não bloqueia (sucesso zera).
#[test]
fn success_resets_counter() {
    let a = ip("10.0.0.1");
    let mut l = AuthLimiter::new(5, 60, 300);
    for i in 0..4u64 {
        l.record_failure(a, 1000 + i);
    }
    l.record_success(a);
    for i in 0..4u64 {
        assert!(!l.record_failure(a, 1100 + i), "sucesso zera o contador");
    }
    assert!(!l.is_blocked(a, 1200), "4+4 falhas após o sucesso não bloqueiam");
}

/// UT-1.24 — bloquear A não afeta B.
#[test]
fn other_ip_not_affected() {
    let (a, b) = (ip("10.0.0.1"), ip("10.0.0.2"));
    let mut l = AuthLimiter::new(5, 60, 300);
    for i in 0..5u64 {
        l.record_failure(a, 1000 + i);
    }
    assert!(l.is_blocked(a, 1010), "A bloqueado");
    assert!(!l.is_blocked(b, 1010), "B livre");
}

/// UT-1.24 — falhas espaçadas > 60 s: a janela desliza e não bloqueia.
#[test]
fn window_slides() {
    let a = ip("10.0.0.1");
    let mut l = AuthLimiter::new(5, 60, 300);
    for i in 0..5u64 {
        assert!(!l.record_failure(a, 1000 + i * 61), "falhas espaçadas não acumulam");
    }
    assert!(!l.is_blocked(a, 2500), "sem 5 falhas dentro da janela");
}

/// Taxa 20/10 s: acima do limite é recusado; após a janela volta;
/// outro IP não é afetado.
#[test]
fn conn_rate_limits_per_ip() {
    let (a, b) = (ip("10.0.0.1"), ip("10.0.0.2"));
    let mut l = ConnRateLimiter::new(20, 10);
    for _ in 0..20 {
        assert!(l.allow(a, 0), "dentro do limite");
    }
    assert!(!l.allow(a, 1), "acima do limite é recusado");
    assert!(l.allow(b, 1), "outro IP não é afetado");
    assert!(l.allow(a, 11), "após a janela volta ao normal");
}

/// 10 000 IPs distintos: depois que o tempo passa, o mapa esvazia
/// (memória limitada, sem crescer sem limite).
#[test]
fn limiter_memory_bounded() {
    let mk = |i: u16| IpAddr::from([10u8, 0, (i >> 8) as u8, i as u8]);

    let mut l = AuthLimiter::new(5, 60, 300);
    for i in 0..10_000u16 {
        l.record_failure(mk(i), 1000);
    }
    assert_eq!(l.len(), 10_000, "10 000 IPs registrados");
    l.prune(1000 + 120); // janela + bloqueio já passaram
    assert!(l.is_empty(), "mapa de falhas esvazia após a limpeza");

    let mut c = ConnRateLimiter::new(20, 10);
    for i in 0..10_000u16 {
        assert!(c.allow(mk(i), 1000));
    }
    assert_eq!(c.len(), 10_000, "10 000 IPs registrados");
    c.prune(1000 + 20); // janela já passou
    assert!(c.is_empty(), "mapa de conexões esvazia após a limpeza");
}

/// ST-1.02 — 6 tentativas com token errado: as 5 primeiras recebem
/// `bad_token` (uniforme); a 6ª é fechada sem resposta; evento
/// `AuthBlocked`; após o bloqueio, o token válido é aceito de novo.
#[tokio::test]
async fn bruteforce_6_attempts_per_minute_blocked() {
    // `auto_approve` só afeta o token válido (checado antes da aprovação):
    // as 5 tentativas erradas seguem recebendo `bad_token`.
    let (mut handle, port, _dir) = boot(true, 2).await; // bloqueio de 2 s (pequeno)
    for _ in 0..5 {
        let mut c = TestClient::connect(port).await.expect("connect");
        c.send(&TestClient::hello_pair(&"f".repeat(32), "dev-x"))
            .await
            .expect("send");
        let w = take_welcome(&mut c).await;
        assert!(!w.accepted, "token errado é recusado");
        assert_eq!(w.reason, Some(Reason::BadToken), "resposta uniforme");
        assert!(c.recv(TIMEOUT).await.is_none(), "socket fecha após a recusa");
    }

    // 6ª: IP bloqueado → fechada no accept, sem resposta.
    let mut c6 = TestClient::connect(port).await.expect("connect");
    let _ = c6.send(&TestClient::hello_pair(&"f".repeat(32), "dev-x")).await;
    assert!(c6.recv(TIMEOUT).await.is_none(), "6ª conexão é fechada sem resposta");

    // Evento `AuthBlocked` com o IP certo (o token nunca aparece).
    let mut blocked_ip = None;
    for _ in 0..16 {
        let ev = handle.events().recv().await.expect("canal aberto");
        if let ServerEvent::AuthBlocked { ip } = ev {
            blocked_ip = Some(ip);
            break;
        }
    }
    assert_eq!(blocked_ip.as_deref(), Some("127.0.0.1"), "AuthBlocked do IP certo");

    // Após o bloqueio expirar (2 s), o token válido volta a ser aceito.
    tokio::time::sleep(Duration::from_millis(2300)).await;
    let mut c7 = TestClient::connect(port).await.expect("connect");
    c7.send(&TestClient::hello_pair(&pair_token(&handle), "dev-x"))
        .await
        .expect("send");
    assert!(take_welcome(&mut c7).await.accepted, "após o bloqueio volta ao normal");

    handle.shutdown().await;
}

/// FT-1.07 — 50 conexões em rajada (limite 20/10 s): ≥ 30 fechadas com
/// `RateLimited`; o servidor segue operando; após a janela volta ao
/// normal (pareamento válido aceito).
#[tokio::test]
async fn connection_storm_limited() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cfg = ServerConfig::for_tests(dir.path().to_path_buf(), true);
    cfg.conn_rate_max = 20;
    cfg.conn_rate_window = 10;
    cfg.max_unauth_conns = 50; // isola o teste do limite de não autenticadas
    cfg.hello_timeout = Duration::from_secs(30); // as 20 que passam esperam no hello
    let mut handle = start(cfg).await.expect("start");
    let port = match handle.events().recv().await.expect("evento") {
        ServerEvent::Listening { port } => port,
        other => panic!("esperava Listening, veio {other:?}"),
    };

    // 50 conexões em sequência (rajada).
    let mut streams = Vec::with_capacity(50);
    for _ in 0..50 {
        streams.push(
            tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .expect("connect"),
        );
    }

    // ≥ 30 fechadas com `RateLimited` (as 20 primeiras passaram ao hello).
    let mut rate_limited = 0usize;
    let deadline = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(deadline);
    while rate_limited < 30 {
        tokio::select! {
            ev = handle.events().recv() => match ev.expect("canal aberto") {
                ServerEvent::ConnectionClosed { reason: CloseReason::RateLimited, .. } => {
                    rate_limited += 1;
                }
                ServerEvent::ConnectionOpened { .. } => {}
                other => panic!("evento inesperado: {other:?}"),
            },
            _ = &mut deadline => panic!("timeout: rate_limited={rate_limited}"),
        }
    }
    assert!(rate_limited >= 30, "50 - 20 do limite");

    // Enquanto a janela não esvazia, conexão nova segue sendo recusada.
    let probe = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect do probe");
    match handle.events().recv().await.expect("evento") {
        ServerEvent::ConnectionClosed { reason, .. } => {
            assert_eq!(reason, CloseReason::RateLimited, "dentro da janela: recusa");
        }
        other => panic!("esperava ConnectionClosed(RateLimited), veio {other:?}"),
    }
    drop(probe);

    // As 20 que passaram fecham ao dropparem; o servidor segue de pé.
    streams.clear();

    // Após a janela (10 s), o servidor volta ao normal.
    tokio::time::sleep(Duration::from_secs(11)).await;
    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&pair_token(&handle), "dev-storm"))
        .await
        .expect("send");
    assert!(take_welcome(&mut c).await.accepted, "após a janela volta ao normal");

    handle.shutdown().await;
    drop(dir);
}

/// Falhas de A não bloqueiam B: B (outro IP de origem, ainda em
/// loopback — `127.0.0.2`) autentica e é aceito.
#[tokio::test]
async fn valid_device_not_blocked_by_failures_of_other_ip() {
    let (mut handle, port, _dir) = boot(true, 2).await;

    // A (127.0.0.1): 5 tokens errados → entra em bloqueio.
    for _ in 0..5 {
        let mut c = TestClient::connect(port).await.expect("connect A");
        c.send(&TestClient::hello_pair(&"f".repeat(32), "dev-a"))
            .await
            .expect("send");
        assert_eq!(
            take_welcome(&mut c).await.reason,
            Some(Reason::BadToken),
            "A recebe bad_token nas 5 tentativas"
        );
    }

    // B (127.0.0.2): token válido → aceito, sem efeito das falhas de A.
    let mut c = TestClient::connect_from("127.0.0.2:0".parse().expect("src"), port)
        .await
        .expect("connect B");
    c.send(&TestClient::hello_pair(&pair_token(&handle), "dev-b"))
        .await
        .expect("send");
    assert!(take_welcome(&mut c).await.accepted, "B não é afetado pelas falhas de A");

    handle.shutdown().await;
}
