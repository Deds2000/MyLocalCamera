//! Testes de fumaça do `mock-sender` contra o `lc-server` real.

use std::path::PathBuf;

use clap::Parser;
use lc_server::config::ServerConfig;
use lc_server::events::ServerEvent;
use lc_server::{start, ServerHandle};
use mock_sender::cli::Cli;
use mock_sender::faults::Faults;
use mock_sender::session::{run, MockConfig};
use mock_sender::state_file;

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

/// Sobe um servidor com `auto_approve` e devolve a porta e o token.
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

/// Token de pareamento atual.
fn pair_token(handle: &ServerHandle) -> String {
    handle
        .trust()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .pair_token()
        .to_owned()
}

/// Configuração básica do mock.
fn config(port: u16, token: Option<String>) -> MockConfig {
    MockConfig {
        hosts: vec!["127.0.0.1".to_owned()],
        port,
        token,
        device_id: "mock-smoke".to_owned(),
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

/// IT `pair_and_send_two_seconds`: ~60 quadros e ~200 blocos em 2 s.
#[tokio::test]
async fn pair_and_send_two_seconds() {
    let (mut handle, port, _dir) = boot().await;
    let token = pair_token(&handle);

    let mut cfg = config(port, Some(token));
    cfg.file = Some(fixture("hd30.h264"));
    cfg.audio = true;
    cfg.seconds = Some(2.0);

    let summary = run(cfg).await.expect("mock deve concluir");
    assert!(
        (57..=63).contains(&summary.video_frames),
        "quadros de vídeo: {} (esperado ~60)",
        summary.video_frames
    );
    assert!(
        (190..=210).contains(&summary.audio_blocks),
        "blocos de áudio: {} (esperado ~200)",
        summary.audio_blocks
    );

    let mut saw_session = false;
    while let Ok(event) = handle.events().try_recv() {
        if matches!(event, ServerEvent::SessionStarted { .. }) {
            saw_session = true;
        }
    }
    assert!(saw_session, "ServerEvent::SessionStarted deve ser emitido");
    handle.shutdown().await;
}

/// IT `reconnect_with_state_file`: a 2ª execução autentica com a chave
/// salva e o servidor não emite `deviceKey` de novo.
#[tokio::test]
async fn reconnect_with_state_file() {
    let (mut handle, port, dir) = boot().await;
    let token = pair_token(&handle);
    let state_path = dir.path().join("mock-state.json");

    // 1ª execução: pareia e grava o estado.
    let mut first = config(port, Some(token));
    first.device_id = "mock-reconnect".to_owned();
    first.state_file = Some(state_path.clone());
    first.seconds = Some(0.2);
    run(first).await.expect("1ª execução");
    let saved = state_file::load(&state_path).expect("estado salvo após parear");
    assert_eq!(saved.device_id, "mock-reconnect");

    // Dá tempo do servidor encerrar a sessão anterior.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 2ª execução: SEM token, só o arquivo de estado (parse da CLI).
    let args = [
        "mock-sender",
        "--host",
        "127.0.0.1",
        "--port",
        &port.to_string(),
        "--state-file",
        &state_path.display().to_string(),
        "--seconds",
        "0.2",
    ];
    let cli = Cli::try_parse_from(args).expect("CLI da 2ª execução");
    let cfg = cli.into_config().expect("config da 2ª execução");
    assert!(cfg.device_key.is_some(), "deve reutilizar a chave salva");
    run(cfg).await.expect("2ª execução (device)");

    let after = state_file::load(&state_path).expect("estado continua");
    assert_eq!(
        after, saved,
        "device auth não pode emitir um deviceKey novo"
    );

    let mut sessions = 0;
    while let Ok(event) = handle.events().try_recv() {
        if matches!(event, ServerEvent::SessionStarted { .. }) {
            sessions += 1;
        }
    }
    assert!(sessions >= 2, "duas sessões aceitas (pareamento + device)");
    handle.shutdown().await;
}
