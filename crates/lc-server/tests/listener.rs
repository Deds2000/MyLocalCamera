//! Testes de integração do listener de `lc-server`.
//!
//! Cobre: porta livre (IT), fallback de porta ocupada (FT-1.05),
//! slowloris/`HelloTimeout` (FT-1.01), limite de não autenticadas (FT-1.02),
//! magic inválido e payload acima do limite (IT) e shutdown (IT).

use std::time::{Duration, Instant};

use lc_protocol::types::MAGIC;
use lc_server::config::ServerConfig;
use lc_server::events::{CloseReason, ServerEvent};
use lc_server::start;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

fn test_config() -> ServerConfig {
    ServerConfig::for_tests(std::env::temp_dir().join("lc-server-test"))
}

async fn next_event(rx: &mut mpsc::Receiver<ServerEvent>) -> ServerEvent {
    rx.recv().await.expect("servidor deveria emitir evento")
}

/// Consome o `Listening{port}` inicial e devolve a porta.
async fn take_listening(rx: &mut mpsc::Receiver<ServerEvent>) -> u16 {
    match next_event(rx).await {
        ServerEvent::Listening { port } => port,
        other => panic!("esperava Listening, veio {other:?}"),
    }
}

fn assert_opened(ev: ServerEvent) {
    match ev {
        ServerEvent::ConnectionOpened { .. } => {}
        other => panic!("esperava ConnectionOpened, veio {other:?}"),
    }
}

fn assert_closed_with(ev: ServerEvent, expected: &CloseReason) {
    match ev {
        ServerEvent::ConnectionClosed { reason, .. } => {
            assert_eq!(&reason, expected, "motivo do fechamento");
        }
        other => panic!("esperava ConnectionClosed, veio {other:?}"),
    }
}

/// Espera `count` fechamentos com o motivo `expected` (na ordem, sem
/// tolerar outros eventos) antes do `timeout`.
async fn wait_closed(
    rx: &mut mpsc::Receiver<ServerEvent>,
    expected: &CloseReason,
    count: usize,
    timeout: Duration,
) {
    let mut seen = 0usize;
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);
    while seen < count {
        tokio::select! {
            ev = rx.recv() => {
                let ev = ev.expect("canal aberto");
                assert_closed_with(ev, expected);
                seen += 1;
            }
            _ = &mut deadline => panic!("timeout: vi {seen}/{count} fechamentos {expected:?}"),
        }
    }
}

// IT: `port: 0` devolve `Listening{port > 0}`.
#[tokio::test]
async fn starts_on_free_port() {
    lc_testkit::init_tracing();
    let mut handle = start(test_config()).await.expect("start deveria funcionar");
    let ev = next_event(handle.events()).await;
    match ev {
        ServerEvent::Listening { port } => assert!(port > 0, "porta aleatória deve ser > 0"),
        other => panic!("esperava Listening, veio {other:?}"),
    }
    handle.shutdown().await;
}

// FT-1.05: porta ocupada → fallback e `PortChanged{requested, actual != requested}`.
#[tokio::test]
async fn port_in_use_falls_back() {
    lc_testkit::init_tracing();
    let blocker = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind do ocupante");
    let requested = blocker.local_addr().expect("addr").port();
    let mut cfg = test_config();
    cfg.port = requested;

    let mut handle = start(cfg).await.expect("start deveria funcionar");
    let ev1 = next_event(handle.events()).await;
    let ev2 = next_event(handle.events()).await;
    let actual = match (ev1, ev2) {
        (
            ServerEvent::PortChanged {
                requested: req,
                actual,
            },
            ServerEvent::Listening { port },
        ) => {
            assert_eq!(req, requested, "porta pedida");
            assert_ne!(actual, requested, "porta efetiva deve diferir da pedida");
            assert_eq!(port, actual, "Listening deve usar a porta efetiva");
            actual
        }
        (a, b) => panic!("eventos inesperados: {a:?} e {b:?}"),
    };
    assert_eq!(handle.local_port(), actual);
    handle.shutdown().await;
}

// FT-1.01: conexão que não envia nada fecha com `HelloTimeout` em ~300 ms.
#[tokio::test]
async fn slowloris_closes_in_hello_timeout() {
    lc_testkit::init_tracing();
    let mut handle = start(test_config()).await.expect("start"); // hello_timeout = 300 ms
    take_listening(handle.events()).await;
    let port = handle.local_port();
    let _slow = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");

    let t0 = Instant::now();
    let ev1 = next_event(handle.events()).await;
    let ev2 = next_event(handle.events()).await;
    let elapsed = t0.elapsed();
    assert_opened(ev1);
    assert_closed_with(ev2, &CloseReason::HelloTimeout);
    assert!(
        elapsed >= Duration::from_millis(200),
        "fechou antes do timeout: {elapsed:?}"
    );
    assert!(
        elapsed <= Duration::from_secs(3),
        "fechou muito depois do timeout: {elapsed:?}"
    );
    handle.shutdown().await;
}

// FT-1.02: limite 8 de não autenticadas; 20 conexões → 12 `TooManyUnauth`;
// depois de fecharem, o servidor segue aceitando.
#[tokio::test]
async fn unauth_limit_enforced() {
    lc_testkit::init_tracing();
    let mut cfg = test_config();
    cfg.hello_timeout = Duration::from_secs(30); // não deixar HelloTimeout soltar permits no meio
    let mut handle = start(cfg).await.expect("start");
    take_listening(handle.events()).await;
    let port = handle.local_port();

    let mut conns = Vec::with_capacity(20);
    for _ in 0..20 {
        conns.push(
            TcpStream::connect(("127.0.0.1", port))
                .await
                .expect("connect"),
        );
    }

    let mut opened = 0usize;
    let mut too_many = 0usize;
    let deadline = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(deadline);
    while too_many < 12 {
        tokio::select! {
            ev = handle.events().recv() => {
                let ev = ev.expect("canal aberto");
                match ev {
                    ServerEvent::ConnectionOpened { .. } => opened += 1,
                    ServerEvent::ConnectionClosed {
                        reason: CloseReason::TooManyUnauth,
                        ..
                    } => too_many += 1,
                    other => panic!("evento inesperado: {other:?}"),
                }
            }
            _ = &mut deadline => panic!("timeout: opened={opened} too_many={too_many}"),
        }
    }
    assert_eq!(too_many, 12, "20 conexões - 8 no limite");
    assert_eq!(opened, 8, "limite de 8 não autenticadas");

    conns.clear();
    wait_closed(
        handle.events(),
        &CloseReason::PeerClosed,
        8,
        Duration::from_secs(5),
    )
    .await;

    // o servidor segue aceitando conexões novas
    let again = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    assert_opened(next_event(handle.events()).await);
    let _ = again;
    handle.shutdown().await;
}

// IT: 18 bytes com magic errado → `ProtocolError`.
#[tokio::test]
async fn bad_magic_closes() {
    lc_testkit::init_tracing();
    let mut cfg = test_config();
    cfg.hello_timeout = Duration::from_secs(10);
    let mut handle = start(cfg).await.expect("start");
    take_listening(handle.events()).await;
    let port = handle.local_port();
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");

    let bad = vec![0u8; 18]; // magic 0x0000 (esperado 0x4C43)
    stream.write_all(&bad).await.expect("write");
    stream.flush().await.expect("flush");

    assert_opened(next_event(handle.events()).await);
    assert_closed_with(
        next_event(handle.events()).await,
        &CloseReason::ProtocolError,
    );
    handle.shutdown().await;
}

// IT: cabeçalho com `length = 8 MiB + 1` → `ProtocolError`; o servidor
// segue aceitando conexões.
#[tokio::test]
async fn too_large_closes() {
    lc_testkit::init_tracing();
    let mut cfg = test_config();
    cfg.hello_timeout = Duration::from_secs(10);
    let mut handle = start(cfg).await.expect("start");
    take_listening(handle.events()).await;
    let port = handle.local_port();
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");

    let mut header = vec![0u8; 18];
    header[0..2].copy_from_slice(&MAGIC.to_le_bytes());
    header[2] = 0x10; // VIDEO
    header[14..18].copy_from_slice(&((8 * 1024 * 1024u32) + 1).to_le_bytes());
    stream.write_all(&header).await.expect("write");
    stream.flush().await.expect("flush");

    assert_opened(next_event(handle.events()).await);
    assert_closed_with(
        next_event(handle.events()).await,
        &CloseReason::ProtocolError,
    );

    let again = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    assert_opened(next_event(handle.events()).await);
    let _ = again;
    handle.shutdown().await;
}

// IT: 3 conexões abertas + `shutdown()` → todas fechadas com `Shutdown`.
#[tokio::test]
async fn shutdown_closes_all() {
    lc_testkit::init_tracing();
    let mut cfg = test_config();
    cfg.hello_timeout = Duration::from_secs(10);
    let mut handle = start(cfg).await.expect("start");
    take_listening(handle.events()).await;
    let port = handle.local_port();

    let mut streams = Vec::with_capacity(3);
    for _ in 0..3 {
        streams.push(
            TcpStream::connect(("127.0.0.1", port))
                .await
                .expect("connect"),
        );
    }
    let mut opened = 0usize;
    while opened < 3 {
        assert_opened(next_event(handle.events()).await);
        opened += 1;
    }

    handle.shutdown().await;
    wait_closed(
        handle.events(),
        &CloseReason::Shutdown,
        3,
        Duration::from_secs(5),
    )
    .await;
    let _ = streams;
}
