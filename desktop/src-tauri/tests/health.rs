use desktop_lib::{health::HealthInfo, server::AppState};
use std::net::TcpListener;

fn temp_state(path: &std::path::Path) -> AppState {
    AppState::new(path.to_owned(), "health-test".to_owned())
}

#[tokio::test]
async fn occupied_requested_port_uses_alternate_and_maps_health() {
    let _occupied = TcpListener::bind(("0.0.0.0", 48_650)).expect("reserve requested port");
    let dir = tempfile::tempdir().unwrap();
    let state = temp_state(dir.path());
    let server = state
        .start_server()
        .await
        .expect("fallback listener starts");
    let effective = server.local_port();
    assert_ne!(effective, state.requested_port);
    state.install_server(server);
    let health = HealthInfo::from_state(&state);
    assert_eq!(health.server.state, "altPort");
    assert_eq!(health.server.port, 48_650);
    assert_eq!(health.server.effective_port, effective);
    state.stop_server().await;
}

#[tokio::test]
async fn failed_start_reports_zero_then_reopens_after_path_is_fixed() {
    let dir = tempfile::tempdir().unwrap();
    let bad_path = dir.path().join("not-a-directory");
    std::fs::write(&bad_path, "x").unwrap();
    let state = temp_state(&bad_path);
    assert!(state.start_server().await.is_err());
    let failed = HealthInfo::from_state(&state);
    assert_eq!(failed.server.state, "failed");
    assert_eq!(failed.server.effective_port, 0);

    std::fs::remove_file(&bad_path).unwrap();
    std::fs::create_dir(&bad_path).unwrap();
    let server = state.start_server().await.expect("restart succeeds");
    let port = server.local_port();
    state.install_server(server);
    assert!(port > 0);
    assert_ne!(HealthInfo::from_state(&state).server.state, "failed");
    state.stop_server().await;
}

#[test]
fn health_mapping_includes_failed_contract() {
    let dir = tempfile::tempdir().unwrap();
    let state = temp_state(dir.path());
    let health = HealthInfo::from_state(&state);
    assert_eq!(health.server.state, "failed");
    assert_eq!(health.server.port, 48_650);
    assert_eq!(health.server.effective_port, 0);
    let json = serde_json::to_value(health).unwrap();
    assert_eq!(json["server"]["effectivePort"], 0);
    assert_eq!(json["stubs"][0], "em breve");
}
