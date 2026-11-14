use super::{
    CommandError, MAX_DEVICE_ID_BYTES, connection_info, mask_pair_token, pairing_uri,
    validate_device_id, validate_pending_request,
};
use crate::server::{AppState, REQUESTED_PORT, take_server};

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
