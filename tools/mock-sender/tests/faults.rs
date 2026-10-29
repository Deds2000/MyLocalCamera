//! AT/FT/IT de falha (F1-T11b): o mock determinístico contra o
//! `lc-server` real, em `127.0.0.1` e porta 0.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use lc_protocol::json::Reason;
use lc_server::config::ServerConfig;
use lc_server::events::{CloseReason, ServerEvent};
use lc_server::sessions::SessionCounters;
use lc_server::{start, ServerHandle};
use mock_sender::faults::Faults;
use mock_sender::session::{run, MockConfig, MockError, Summary};

/// Caminho de um fixture (falha explícita se não existir).
fn fixture(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name);
    assert!(
        path.exists(),
        "fixture {} não encontrada: rode scripts/make-fixtures.ps1",
        path.display()
    );
    path
}

/// Configuração base: `auto_approve` e limite de conexões alto.
fn base_config() -> ServerConfig {
    let mut cfg = ServerConfig::for_tests(std::env::temp_dir(), true);
    cfg.conn_rate_max = 10_000;
    cfg
}

/// Sobe o servidor e devolve a porta e o diretório temporário.
async fn boot(mut cfg: ServerConfig) -> (ServerHandle, u16, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    cfg.data_dir = dir.path().to_path_buf();
    let mut handle = start(cfg).await.expect("start");
    let port = match handle.events().recv().await.expect("evento") {
        ServerEvent::Listening { port } => port,
        other => panic!("esperava Listening, veio {other:?}"),
    };
    (handle, port, dir)
}

/// Token de pareamento atual (rotaciona após cada pareamento).
fn pair_token(handle: &ServerHandle) -> String {
    handle
        .trust()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .pair_token()
        .to_owned()
}

/// Configuração básica do mock (sem mídia e sem falhas).
fn mock(port: u16, token: String) -> MockConfig {
    MockConfig {
        hosts: vec!["127.0.0.1".to_owned()],
        port,
        token: Some(token),
        device_id: "mock-faults".to_owned(),
        device_name: "Mock Phone".to_owned(),
        device_key: None,
        state_file: None,
        file: None,
        loop_video: false,
        fps: 30,
        audio: false,
        tone: 440.0,
        seconds: None,
        faults: Faults::default(),
    }
}

/// Espera um evento que satisfaça `pred` (ou falha por timeout).
async fn wait_event(
    handle: &mut ServerHandle,
    pred: impl Fn(&ServerEvent) -> bool,
    timeout: Duration,
) -> ServerEvent {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "timeout esperando evento");
        match tokio::time::timeout(remaining, handle.events().recv()).await {
            Ok(Some(event)) if pred(&event) => return event,
            Ok(Some(_)) => {}
            Ok(None) => panic!("canal de eventos fechou"),
            Err(_) => panic!("timeout esperando evento"),
        }
    }
}

/// Espera o `SessionEnded` e devolve o motivo.
async fn wait_session_ended(handle: &mut ServerHandle, timeout: Duration) -> CloseReason {
    match wait_event(
        handle,
        |e| matches!(e, ServerEvent::SessionEnded { .. }),
        timeout,
    )
    .await
    {
        ServerEvent::SessionEnded { reason, .. } => reason,
        other => unreachable!("filtro garantiu SessionEnded: {other:?}"),
    }
}

/// Espera o `SessionStarted` e devolve os contadores da sessão.
async fn wait_session_started(
    handle: &mut ServerHandle,
    timeout: Duration,
) -> (String, Arc<SessionCounters>) {
    match wait_event(
        handle,
        |e| matches!(e, ServerEvent::SessionStarted { .. }),
        timeout,
    )
    .await
    {
        ServerEvent::SessionStarted { session_id, .. } => {
            let counters = handle
                .sessions()
                .counters(&session_id)
                .expect("contadores da sessão iniciada");
            (session_id, counters)
        }
        other => unreachable!("filtro garantiu SessionStarted: {other:?}"),
    }
}

/// AT-1.07: `--fake-version 2.0` é recusado com `version_mismatch`.
#[tokio::test]
async fn fake_version_denied() {
    let (mut handle, port, _dir) = boot(base_config()).await;
    let mut cfg = mock(port, pair_token(&handle));
    cfg.faults.fake_version = Some((2, 0));

    let error = run(cfg).await.expect_err("versão 2.0 deve ser recusada");
    assert!(
        matches!(error, MockError::Denied(Some(Reason::VersionMismatch))),
        "erro inesperado: {error:?}"
    );
    assert_eq!(error.exit_code(), 2);
    handle.shutdown().await;
}

/// AT-1.10: sem `PONG`, o servidor encerra por `ConnectionLost`.
#[tokio::test]
async fn no_pong_session_ends() {
    let mut server = base_config();
    server.ping_interval = Duration::from_millis(100);
    server.liveness_timeout = Duration::from_millis(400);
    let (mut handle, port, _dir) = boot(server).await;

    let mut cfg = mock(port, pair_token(&handle));
    cfg.faults.no_pong = true;
    cfg.seconds = Some(2.0);
    let _summary = run(cfg).await.expect("mock conclui após o servidor fechar");

    let reason = wait_session_ended(&mut handle, Duration::from_secs(2)).await;
    assert_eq!(reason, CloseReason::ConnectionLost);
    handle.shutdown().await;
}

/// AT-1.11: `length` acima do limite → servidor fecha com `ProtocolError`.
#[tokio::test]
async fn huge_length_protocol_error() {
    let (mut handle, port, _dir) = boot(base_config()).await;
    let mut cfg = mock(port, pair_token(&handle));
    cfg.faults.huge_length = true;
    cfg.seconds = Some(2.0);

    let summary = run(cfg).await.expect("mock lê até o servidor fechar");
    assert_eq!(summary.closed_by.as_deref(), Some("server"));
    assert!(summary.closed_after_ms.is_some());

    match wait_event(
        &mut handle,
        |e| matches!(e, ServerEvent::ConnectionClosed { .. }),
        Duration::from_secs(2),
    )
    .await
    {
        ServerEvent::ConnectionClosed { reason, .. } => {
            assert_eq!(reason, CloseReason::ProtocolError);
        }
        other => unreachable!("filtro garantiu ConnectionClosed: {other:?}"),
    }
    handle.shutdown().await;
}

/// AT-1.12: quadro 0x99 no meio do vídeo não derruba a sessão.
#[tokio::test]
async fn unknown_type_midstream_continues() {
    let (mut handle, port, _dir) = boot(base_config()).await;
    let mut cfg = mock(port, pair_token(&handle));
    cfg.file = Some(fixture("hd30.h264"));
    cfg.loop_video = true;
    cfg.faults.unknown_type = true;
    cfg.seconds = Some(3.0);

    let task = tokio::spawn(run(cfg));
    let (_session, counters) = wait_session_started(&mut handle, Duration::from_secs(5)).await;
    let summary = task.await.expect("task").expect("mock conclui");
    assert!(
        (80..=95).contains(&summary.video_frames),
        "quadros enviados: {} (esperado ~90)",
        summary.video_frames
    );

    let reason = wait_session_ended(&mut handle, Duration::from_secs(2)).await;
    assert!(
        matches!(
            reason,
            CloseReason::PeerClosed | CloseReason::ConnectionLost
        ),
        "sessão caiu por {reason:?} (o quadro desconhecido não pode derrubar)"
    );

    // O servidor contou os quadros de vídeo normalmente.
    tokio::time::sleep(Duration::from_millis(100)).await;
    let server_frames = counters.snapshot().video_frames[0];
    assert!(
        server_frames + 5 >= summary.video_frames,
        "servidor viu {server_frames} quadros; mock enviou {}",
        summary.video_frames
    );
    handle.shutdown().await;
}

/// AT-1.14: JSON inválido fecha a conexão, mas o servidor continua vivo.
#[tokio::test]
async fn bad_json_closes_but_server_lives() {
    let (mut handle, port, _dir) = boot(base_config()).await;
    let mut cfg = mock(port, pair_token(&handle));
    cfg.faults.bad_json = true;
    cfg.seconds = Some(2.0);

    let summary = run(cfg).await.expect("mock lê até o servidor fechar");
    assert_eq!(summary.closed_by.as_deref(), Some("server"));
    match wait_event(
        &mut handle,
        |e| matches!(e, ServerEvent::ConnectionClosed { .. }),
        Duration::from_secs(2),
    )
    .await
    {
        ServerEvent::ConnectionClosed { reason, .. } => {
            assert_eq!(reason, CloseReason::ProtocolError);
        }
        other => unreachable!("filtro garantiu ConnectionClosed: {other:?}"),
    }

    // Um segundo mock pareia normalmente.
    let mut ok = mock(port, pair_token(&handle));
    ok.seconds = Some(0.3);
    run(ok).await.expect("segundo mock deve parear");
    let _ = wait_session_started(&mut handle, Duration::from_secs(2)).await;
    handle.shutdown().await;
}

/// IT-1.03: 20 reconexões, sem vazamento de conexões/sessões.
#[tokio::test]
async fn reconnect_20_no_leak() {
    let (mut handle, port, dir) = boot(base_config()).await;
    let token = pair_token(&handle);
    let mut cfg = mock(port, token);
    cfg.file = Some(fixture("pattern43.h264"));
    cfg.loop_video = true;
    cfg.state_file = Some(dir.path().join("mock-state.json"));
    cfg.faults.reconnect = Some(20);

    let task = tokio::spawn(run(cfg));
    let mut started = 0u32;
    let mut ended = 0u32;
    while !task.is_finished() {
        tokio::select! {
            event = handle.events().recv() => match event {
                Some(ServerEvent::SessionStarted { .. }) => started += 1,
                Some(ServerEvent::SessionEnded { .. }) => ended += 1,
                Some(_) => {}
                None => break,
            },
            () = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }
    let summary = task.await.expect("task do mock").expect("20 reconexões");
    while let Ok(event) = handle.events().try_recv() {
        match event {
            ServerEvent::SessionStarted { .. } => started += 1,
            ServerEvent::SessionEnded { .. } => ended += 1,
            _ => {}
        }
    }
    assert_eq!(summary.sessions, 20);
    assert_eq!(started, 20, "20 sessões iniciadas");
    assert!(ended >= 19, "sessões encerradas: {ended}");

    // O servidor fecha tudo; nenhuma conexão fica pendurada.
    for _ in 0..100 {
        if handle.active_connections() == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(handle.active_connections(), 0, "conexões vazadas");
    handle.shutdown().await;
}

/// Executa o fixture inteiro (sem loop) e devolve os totais do servidor.
async fn run_pattern43(fragment: Option<usize>) -> (Summary, u64, u64) {
    let (mut handle, port, _dir) = boot(base_config()).await;
    let mut cfg = mock(port, pair_token(&handle));
    cfg.file = Some(fixture("pattern43.h264"));
    cfg.seconds = Some(10.0);
    cfg.faults.fragment = fragment;

    let task = tokio::spawn(run(cfg));
    let (_session, counters) = wait_session_started(&mut handle, Duration::from_secs(5)).await;
    let summary = task.await.expect("task").expect("mock conclui");
    tokio::time::sleep(Duration::from_millis(100)).await;
    let snapshot = counters.snapshot();
    handle.shutdown().await;
    (summary, snapshot.video_frames[0], snapshot.video_bytes[0])
}

/// IT-1.04: fragmentar em 1 byte não muda os totais recebidos (±2%).
#[tokio::test]
async fn fragment_1_byte_same_totals() {
    let (plain, plain_frames, plain_bytes) = run_pattern43(None).await;
    let (fragmented, frag_frames, frag_bytes) = run_pattern43(Some(1)).await;

    assert_eq!(plain.video_frames, fragmented.video_frames);
    let tolerance = plain_frames / 50 + 1; // ±2%
    assert!(
        plain_frames.abs_diff(frag_frames) <= tolerance,
        "quadros no servidor: {plain_frames} vs {frag_frames}"
    );
    let byte_tolerance = plain_bytes / 50 + 1;
    assert!(
        plain_bytes.abs_diff(frag_bytes) <= byte_tolerance,
        "bytes no servidor: {plain_bytes} vs {frag_bytes}"
    );
}

/// FT-1.01: `HELLO` atrasado além do prazo → `HelloTimeout`.
#[tokio::test]
async fn delay_hello_beyond_timeout() {
    // `for_tests` usa hello_timeout de 300 ms.
    let (mut handle, port, _dir) = boot(base_config()).await;
    let mut cfg = mock(port, pair_token(&handle));
    cfg.faults.delay_hello = Some(Duration::from_millis(1000));
    cfg.seconds = Some(2.0);

    let error = run(cfg).await.expect_err("HELLO atrasado deve falhar");
    assert!(
        matches!(error, MockError::Closed | MockError::Io(_)),
        "erro inesperado: {error:?}"
    );
    match wait_event(
        &mut handle,
        |e| matches!(e, ServerEvent::ConnectionClosed { .. }),
        Duration::from_secs(2),
    )
    .await
    {
        ServerEvent::ConnectionClosed { reason, .. } => {
            assert_eq!(reason, CloseReason::HelloTimeout);
        }
        other => unreachable!("filtro garantiu ConnectionClosed: {other:?}"),
    }
    handle.shutdown().await;
}

/// FT-1.03: queda abrupta no meio do vídeo; estado limpo para reconectar.
#[tokio::test]
async fn kill_midstream_clean() {
    let (mut handle, port, _dir) = boot(base_config()).await;
    let mut cfg = mock(port, pair_token(&handle));
    cfg.file = Some(fixture("hd30.h264"));
    cfg.loop_video = true;
    cfg.faults.kill_after = Some(Duration::from_secs(2));
    cfg.seconds = Some(5.0);

    let summary = run(cfg).await.expect("kill não é erro do mock");
    assert_eq!(summary.closed_by.as_deref(), Some("killed"));
    let reason = wait_session_ended(&mut handle, Duration::from_secs(2)).await;
    assert!(
        matches!(
            reason,
            CloseReason::PeerClosed | CloseReason::ConnectionLost
        ),
        "motivo inesperado: {reason:?}"
    );

    // Estado limpo: um novo mock pareia normalmente.
    let mut again = mock(port, pair_token(&handle));
    again.seconds = Some(0.3);
    run(again).await.expect("novo mock deve parear");
    handle.shutdown().await;
}

/// FT-1.06: stall maior que a liveness → `ConnectionLost`.
#[tokio::test]
async fn stall_beyond_liveness_detected() {
    let mut server = base_config();
    server.ping_interval = Duration::from_millis(100);
    server.liveness_timeout = Duration::from_millis(400);
    let (mut handle, port, _dir) = boot(server).await;

    let mut cfg = mock(port, pair_token(&handle));
    cfg.file = Some(fixture("hd30.h264"));
    cfg.loop_video = true;
    cfg.faults.stall = Some((Duration::from_millis(2000), Duration::from_secs(1)));
    cfg.seconds = Some(4.0);

    let _summary = run(cfg).await.expect("mock conclui após o servidor fechar");
    let reason = wait_session_ended(&mut handle, Duration::from_secs(3)).await;
    assert_eq!(reason, CloseReason::ConnectionLost);
    handle.shutdown().await;
}

/// `--slow 400`: a vazão observada fica próxima de 400 kbit/s.
#[tokio::test]
async fn slow_reduces_rate() {
    let (mut handle, port, _dir) = boot(base_config()).await;
    let mut cfg = mock(port, pair_token(&handle));
    cfg.file = Some(fixture("hd30.h264"));
    cfg.loop_video = true;
    cfg.faults.slow_kbps = Some(400);
    cfg.seconds = Some(4.0);

    let started = Instant::now();
    let summary = run(cfg).await.expect("mock conclui");
    let secs = started.elapsed().as_secs_f64().max(0.001);
    let kbps = summary.video_bytes as f64 * 8.0 / secs / 1000.0;
    assert!(
        kbps <= 500.0,
        "vazão observada {kbps:.0} kbit/s (esperado <= ~500)"
    );
    assert!(kbps >= 100.0, "vazão baixa demais: {kbps:.0} kbit/s");
    handle.shutdown().await;
}

/// `--drop-ping`: perder o 1º PING não derruba a sessão.
#[tokio::test]
async fn drop_ping_once_is_tolerated() {
    let mut server = base_config();
    server.ping_interval = Duration::from_millis(100);
    server.liveness_timeout = Duration::from_secs(6);
    let (mut handle, port, _dir) = boot(server).await;

    let mut cfg = mock(port, pair_token(&handle));
    cfg.faults.drop_ping = true;
    cfg.seconds = Some(1.0);
    let summary = run(cfg).await.expect("mock conclui");
    assert!(summary.closed_by.is_none());

    // O servidor só encerra depois do prazo do mock, por EOF (PeerClosed):
    // se o 1º PING perdido tivesse derrubado a sessão, viria ConnectionLost.
    let reason = wait_session_ended(&mut handle, Duration::from_secs(2)).await;
    assert_eq!(
        reason,
        CloseReason::PeerClosed,
        "o PING descartado não pode derrubar a sessão"
    );
    handle.shutdown().await;
}
