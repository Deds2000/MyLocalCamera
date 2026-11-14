use std::sync::atomic::Ordering;
use std::time::Duration;

use lc_server::events::{ApprovalOutcome, CloseReason, Decision, ServerEvent};
use lc_server::stats::SessionStats;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::server::{AppState, lock_server};

const CONNECTION_EVENT: &str = "session://connection";
const STATS_EVENT: &str = "session://stats";
const PENDING_EVENT: &str = "trust://pending";
const EMPTY_POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionPayload {
    state: &'static str,
    session_id: Option<String>,
    device_id: Option<String>,
    name: Option<String>,
    model: Option<String>,
    transport: Option<&'static str>,
    ip: Option<String>,
    error_code: Option<String>,
    requested_port: Option<u16>,
    effective_port: Option<u16>,
}

impl ConnectionPayload {
    fn state(state: &'static str) -> Self {
        Self {
            state,
            session_id: None,
            device_id: None,
            name: None,
            model: None,
            transport: None,
            ip: None,
            error_code: None,
            requested_port: None,
            effective_port: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PendingPayload {
    request_id: u64,
    device_id: String,
    name: String,
    model: String,
    ip: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsPayload {
    session_id: String,
    fps: f32,
    bitrate_kbps: f32,
    rtt_ms: Option<f32>,
    /// Video frames dropped by the bounded desktop queue since the prior stats event.
    lost: u64,
}

pub(crate) fn spawn<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        let mut last_dropped_frames = 0;
        loop {
            let next = {
                let state = app.state::<AppState>();
                if state.shutting_down.load(Ordering::Acquire) {
                    break;
                }
                let mut server = lock_server(&state);
                match server.as_mut() {
                    Some(server) => server.events().try_recv(),
                    None => break,
                }
            };

            match next {
                Ok(event) => {
                    let state = app.state::<AppState>();
                    dispatch(&app, &state, event, &mut last_dropped_frames);
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    tokio::time::sleep(EMPTY_POLL_INTERVAL).await;
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
            }
        }
    });
}

trait TauriEventSink {
    fn emit<T: Serialize + Clone>(&self, event: &str, payload: T);
}

impl<R: Runtime> TauriEventSink for AppHandle<R> {
    fn emit<T: Serialize + Clone>(&self, event: &str, payload: T) {
        let _ = Emitter::emit(self, event, payload);
    }
}

fn dispatch<E: TauriEventSink>(
    app: &E,
    state: &AppState,
    event: ServerEvent,
    last_dropped_frames: &mut u64,
) {
    match event {
        ServerEvent::Listening { port } => {
            let mut payload = ConnectionPayload::state("listening");
            payload.effective_port = Some(port);
            emit(app, CONNECTION_EVENT, payload);
        }
        ServerEvent::PortChanged { requested, actual } => {
            let mut payload = ConnectionPayload::state("altPort");
            payload.requested_port = Some(requested);
            payload.effective_port = Some(actual);
            emit(app, CONNECTION_EVENT, payload);
        }
        ServerEvent::ConnectionOpened { peer, .. } => {
            let mut payload = ConnectionPayload::state("connecting");
            payload.transport = Some("tcp");
            payload.ip = Some(peer.ip().to_string());
            emit(app, CONNECTION_EVENT, payload);
        }
        ServerEvent::ConnectionClosed { reason, .. } => {
            let mut payload = ConnectionPayload::state(connection_closed_state(&reason));
            payload.error_code = close_reason_code(&reason).map(str::to_owned);
            emit(app, CONNECTION_EVENT, payload);
        }
        ServerEvent::SessionStarted {
            session_id,
            device_id,
            device_name,
            peer,
            ..
        } => {
            let mut payload = ConnectionPayload::state("streaming");
            payload.session_id = Some(session_id);
            payload.device_id = Some(device_id);
            payload.name = Some(device_name);
            payload.transport = Some("tcp");
            payload.ip = Some(peer.ip().to_string());
            emit(app, CONNECTION_EVENT, payload);
        }
        ServerEvent::SessionEnded { session_id, reason } => {
            let mut payload = ConnectionPayload::state(connection_closed_state(&reason));
            payload.session_id = Some(session_id);
            payload.error_code = close_reason_code(&reason).map(str::to_owned);
            emit(app, CONNECTION_EVENT, payload);
        }
        ServerEvent::Stats { session_id, stats } => {
            let total_dropped_frames = {
                lock_server(state)
                    .as_ref()
                    .map_or(*last_dropped_frames, |server| server.dropped_video_frames())
            };
            let lost = total_dropped_frames.saturating_sub(*last_dropped_frames);
            *last_dropped_frames = total_dropped_frames;
            emit(app, STATS_EVENT, stats_payload(session_id, stats, lost));
        }
        ServerEvent::ApprovalRequested(request) => {
            state
                .pending_request_id
                .store(request.request_id, Ordering::Release);
            emit(app, CONNECTION_EVENT, ConnectionPayload::state("pending"));
            emit(
                app,
                PENDING_EVENT,
                PendingPayload {
                    request_id: request.request_id,
                    device_id: request.device_id,
                    name: request.name,
                    model: request.model,
                    ip: request.ip,
                },
            );
        }
        ServerEvent::ApprovalResolved {
            request_id,
            outcome,
        } => {
            let _ = state.pending_request_id.compare_exchange(
                request_id,
                0,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            let (connection_state, error_code) = approval_state(&outcome);
            let mut payload = ConnectionPayload::state(connection_state);
            payload.error_code = error_code.map(str::to_owned);
            emit(app, CONNECTION_EVENT, payload);
        }
        ServerEvent::AuthBlocked { ip } => {
            let mut payload = ConnectionPayload::state("error");
            payload.ip = Some(ip);
            payload.error_code = Some("auth_blocked".to_owned());
            emit(app, CONNECTION_EVENT, payload);
        }
        ServerEvent::AudioOpened { session_id } => {
            let mut payload = ConnectionPayload::state("audioConnected");
            payload.session_id = Some(session_id);
            emit(app, CONNECTION_EVENT, payload);
        }
        ServerEvent::AudioClosed { session_id, reason } => {
            let mut payload = ConnectionPayload::state("streaming");
            payload.session_id = Some(session_id);
            payload.error_code = close_reason_code(&reason).map(str::to_owned);
            emit(app, CONNECTION_EVENT, payload);
        }
        ServerEvent::UsbWindowOpened | ServerEvent::UsbWindowClosed { .. } => {}
    }
}

fn stats_payload(session_id: String, stats: SessionStats, lost: u64) -> StatsPayload {
    StatsPayload {
        session_id,
        fps: stats.video_fps[0],
        bitrate_kbps: stats.video_kbps[0],
        rtt_ms: stats.rtt_ms,
        lost,
    }
}

fn approval_state(outcome: &ApprovalOutcome) -> (&'static str, Option<&'static str>) {
    match outcome {
        ApprovalOutcome::Approved(Decision::ApproveAlways | Decision::ApproveOnce) => {
            ("connecting", None)
        }
        ApprovalOutcome::Approved(Decision::Reject) => ("idle", Some("not_approved")),
        ApprovalOutcome::Rejected => ("idle", Some("not_approved")),
        ApprovalOutcome::TimedOut => ("idle", Some("timeout")),
        ApprovalOutcome::Cancelled => ("idle", Some("cancelled")),
    }
}

fn connection_closed_state(reason: &CloseReason) -> &'static str {
    if matches!(reason, CloseReason::ConnectionLost) {
        "reconnecting"
    } else {
        "idle"
    }
}

fn close_reason_code(reason: &CloseReason) -> Option<&'static str> {
    Some(match reason {
        CloseReason::PeerClosed => "peer_closed",
        CloseReason::HelloTimeout => "timeout",
        CloseReason::ProtocolError => "protocol_error",
        CloseReason::TooManyUnauth => "too_many_unauth",
        CloseReason::Shutdown => "shutdown",
        CloseReason::Replaced => "replaced",
        CloseReason::ConnectionLost => "connection_lost",
        CloseReason::AuthDenied => "auth_denied",
        CloseReason::RateLimited => "rate_limited",
    })
}

fn emit<E: TauriEventSink, T: Serialize + Clone>(app: &E, event: &str, payload: T) {
    app.emit(event, payload);
}

#[cfg(test)]
#[path = "events_pump_tests.rs"]
mod tests;
