use std::sync::atomic::Ordering;

use super::{
    CommandError, MAX_DEVICE_ID_BYTES, connection_info, ensure_server_running,
    install_restarted_server, mask_pair_token, pairing_uri, validate_device_id,
    validate_pending_request,
};
use crate::server::{AppState, REQUESTED_PORT, take_server};

static PORT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[test]
fn rejects_oversized_device_id_without_echoing_input() {
    let invalid = "x".repeat(10 * 1024);
    let error = validate_device_id(&invalid).expect_err("10 KB ID deve ser rejeitado");
    assert_eq!(error, CommandError::InvalidInput);
    assert_eq!(
        serde_json::to_string(&error).expect("serialize"),
        "\"invalid_input\""
    );
    assert!(validate_device_id(&"a".repeat(MAX_DEVICE_ID_BYTES)).is_ok());
}

#[tokio::test]
async fn restart_starts_missing_listener_and_is_idempotent() {
    let _port = PORT_LOCK.lock().await;
    let data_dir = tempfile::tempdir().expect("test data dir");
    let state = AppState::new(data_dir.path().to_owned(), "test".to_owned());
    assert!(ensure_server_running(&state).await.expect("first restart"));
    assert!(
        !ensure_server_running(&state)
            .await
            .expect("idempotent restart")
    );
    if let Some(mut server) = take_server(&state) {
        server.shutdown().await;
    }
}

#[tokio::test]
async fn shutdown_during_restart_does_not_install_new_listener() {
    let _port = PORT_LOCK.lock().await;
    let data_dir = tempfile::tempdir().expect("test data dir");
    let state = AppState::new(data_dir.path().to_owned(), "test".to_owned());
    let server = state
        .start_server()
        .await
        .expect("start temporary listener");
    let port = server.local_port();
    state.shutting_down.store(true, Ordering::Release);
    assert_eq!(
        install_restarted_server(&state, server).await,
        Err(CommandError::ServerUnavailable)
    );
    assert!(crate::server::lock_server(&state).is_none());
    let _rebound = std::net::TcpListener::bind(("0.0.0.0", port))
        .expect("shutdown closes newly started listener");
}

#[tokio::test]
async fn ft_308_reports_alternate_listener_and_matching_qr_port() {
    let _port = PORT_LOCK.lock().await;
    let _occupied =
        std::net::TcpListener::bind(("0.0.0.0", REQUESTED_PORT)).expect("reserve requested port");
    let data_dir = tempfile::tempdir().expect("test data dir");
    let state = AppState::new(data_dir.path().to_owned(), "test".to_owned());
    let server = state
        .start_server()
        .await
        .expect("fallback listener starts");
    let effective = server.local_port();
    assert_ne!(effective, REQUESTED_PORT);
    *crate::server::lock_server(&state) = Some(server);
    let health = crate::health::HealthInfo::from_state(&state);
    assert_eq!(health.server.state, "altPort");
    let info = connection_info(&state).expect("connection info");
    assert_eq!(info.effective_port, effective);
    assert!(
        info.qrcode_uri
            .contains(&format!("&port={effective}&token="))
    );
    assert!(!info.qrcode_uri.contains("&port=48650&"));
    if let Some(mut server) = take_server(&state) {
        server.shutdown().await;
    }
}

#[test]
fn rejects_unknown_approval_request() {
    assert_eq!(
        validate_pending_request(0, 999),
        Err(CommandError::NoPendingApproval)
    );
    assert_eq!(
        validate_pending_request(12, 13),
        Err(CommandError::NoPendingApproval)
    );
}

#[test]
fn pairing_uri_encodes_name_and_mask_never_contains_full_token() {
    let token = "0123456789abcdef0123456789abcdef";
    let uri = pairing_uri(
        &["192.168.1.2".to_owned()],
        48_650,
        token,
        "pc-id",
        "My PC & Camera",
    );
    assert!(uri.contains("name=My%20PC%20%26%20Camera"));
    let masked = mask_pair_token(token);
    assert_eq!(masked, "0123••••");
    assert!(!masked.contains(token));
}

#[tokio::test]
async fn connection_info_reports_live_port_and_pairing_uri() {
    let data_dir = tempfile::tempdir().expect("test data dir");
    let server = lc_server::start(lc_server::config::ServerConfig::for_tests(
        data_dir.path().to_owned(),
        false,
    ))
    .await
    .expect("test server starts");
    let effective_port = server.local_port();
    let state = AppState::from_server(server, REQUESTED_PORT, "Smoke PC & Camera".to_owned());

    let info = connection_info(&state).expect("connection info");
    assert_eq!(info.port, REQUESTED_PORT);
    assert_eq!(info.effective_port, effective_port);
    assert!(info.qrcode_uri.starts_with("localcamera://connect?"));
    assert!(
        info.qrcode_uri
            .contains(&format!("&port={effective_port}&token="))
    );
    assert!(info.qrcode_uri.contains("name=Smoke%20PC%20%26%20Camera"));
    assert_eq!(info.pair_code_masked.chars().count(), 8);

    if let Some(mut server) = take_server(&state) {
        server.shutdown().await;
    }
}
