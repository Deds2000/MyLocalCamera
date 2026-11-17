use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::Duration;

use lc_server::config::ServerConfig;
use lc_server::events::{Decision, ServerEvent};
use mock_sender::config::{MockConfig, MockError};
use mock_sender::faults::Faults;
use serde::Serialize;

use super::{
    TauriEventSink, close_reason_code, connection_closed_state, dispatch, emit_health_if_changed,
};
use crate::server::{AppState, lock_server, take_server};

#[derive(Default)]
struct CapturingSink(Mutex<Vec<(String, serde_json::Value)>>);

impl TauriEventSink for CapturingSink {
    fn emit<T: Serialize + Clone>(&self, event: &str, payload: T) {
        self.0
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push((
                event.to_owned(),
                serde_json::to_value(payload).expect("event payload serializes"),
            ));
    }
}

#[tokio::test]
async fn health_event_emits_only_on_change_with_get_health_payload() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = AppState::new(dir.path().to_owned(), "test".to_owned());
    let sink = CapturingSink::default();
    let mut last = None;

    emit_health_if_changed(&sink, &state, &mut last);
    {
        let events = sink.0.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "health://changed");
        assert_eq!(events[0].1["server"]["state"], "failed");
        assert_eq!(events[0].1["server"]["effectivePort"], 0);
    }

    let server = lc_server::start(ServerConfig::for_tests(dir.path().to_owned(), false))
        .await
        .expect("test listener starts on an ephemeral port");
    let effective_port = server.local_port();
    state.requested_port = effective_port;
    state.install_server(server);
    let expected = crate::health::HealthInfo::from_state(&state);

    emit_health_if_changed(&sink, &state, &mut last);
    emit_health_if_changed(&sink, &state, &mut last);

    {
        let events = sink.0.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].0, "health://changed");
        assert_eq!(events[1].1["server"]["state"], "listening");
        assert_eq!(events[1].1["server"]["effectivePort"], effective_port);
        assert_eq!(
            events[1].1,
            serde_json::to_value(expected).expect("health payload serializes")
        );
    }
    state.stop_server().await;
}

#[test]
fn connection_loss_maps_to_reconnecting_and_stable_code() {
    use lc_server::events::CloseReason;

    assert_eq!(
        connection_closed_state(&CloseReason::ConnectionLost),
        "reconnecting"
    );
    assert_eq!(
        close_reason_code(&CloseReason::ConnectionLost),
        Some("connection_lost")
    );
}

#[test]
fn stats_reports_measured_queue_drops() {
    use super::stats_payload;
    use lc_server::stats::SessionStats;

    let stats = SessionStats {
        video_fps: [30.0, 0.0, 0.0],
        video_kbps: [800.0, 0.0, 0.0],
        audio_blocks_per_s: 0.0,
        rtt_ms: Some(25.0),
        totals: Default::default(),
    };
    let payload = stats_payload("session".to_owned(), stats, 2);
    assert_eq!(payload.fps, 30.0);
    assert_eq!(payload.bitrate_kbps, 800.0);
    assert_eq!(payload.rtt_ms, Some(25.0));
    assert_eq!(payload.lost, 2);
}

#[tokio::test]
async fn mock_sender_pending_event_payload_is_secret_free() {
    let data_dir = tempfile::tempdir().expect("test data dir");
    let server = lc_server::start(ServerConfig::for_tests(data_dir.path().to_owned(), false))
        .await
        .expect("test server starts");
    let port = server.local_port();
    let trust_file = std::fs::read_to_string(data_dir.path().join("trust.json"))
        .expect("server creates trust store");
    let pairing_token = serde_json::from_str::<serde_json::Value>(&trust_file)
        .expect("trust store JSON")
        .get("pairToken")
        .and_then(serde_json::Value::as_str)
        .expect("pair token field")
        .to_owned();

    let state = AppState::from_server(server, port, "F3-T13 test".to_owned());
    let sink = CapturingSink::default();
    let mut last_dropped = 0;
    let mut last_health = None;

    let sender = tokio::spawn(mock_sender::session::run(MockConfig {
        hosts: vec!["127.0.0.1".to_owned()],
        port,
        token: Some(pairing_token.clone()),
        device_id: "f3-t13-mock-device".to_owned(),
        device_name: "Smoke Camera".to_owned(),
        device_key: None,
        state_file: None,
        file: None,
        loop_video: false,
        fps: 30,
        audio: false,
        tone: 440.0,
        seconds: Some(0.1),
        faults: Faults::default(),
    }));

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let next = {
                let mut server = lock_server(&state);
                server.as_mut().expect("server alive").events().try_recv()
            };
            match next {
                Ok(event) => {
                    let is_pending = matches!(&event, ServerEvent::ApprovalRequested(_));
                    dispatch(&sink, &state, event, &mut last_dropped, &mut last_health);
                    if is_pending {
                        break;
                    }
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    panic!("server event channel disconnected")
                }
            }
        }
    })
    .await
    .expect("approval event emitted");

    let emitted = sink
        .0
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clone();
    let (_, payload) = emitted
        .iter()
        .find(|(event, _)| event == "trust://pending")
        .expect("pending event emitted");
    assert_eq!(payload["name"], "Smoke Camera");
    assert_eq!(payload["model"], "Mock");
    assert_eq!(payload["ip"], "127.0.0.1");
    assert!(!payload.to_string().contains(&pairing_token));
    assert!(payload.get("token").is_none());
    assert!(payload.get("deviceKey").is_none());

    let request_id = state.pending_request_id.load(Ordering::Acquire);
    assert_ne!(request_id, 0);
    let decided = lock_server(&state)
        .as_ref()
        .expect("server alive")
        .decide(request_id, Decision::Reject);
    assert!(decided);
    assert!(matches!(
        sender.await.expect("sender task"),
        Err(MockError::Denied(_))
    ));

    state.shutting_down.store(true, Ordering::Release);
    if let Some(mut server) = take_server(&state) {
        server.shutdown().await;
    }
}
