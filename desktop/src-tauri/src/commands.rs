use std::sync::atomic::Ordering;

use lc_server::events::Decision;
use serde::Serialize;
use tauri::State;

use crate::server::{AppState, lock_server};

#[path = "commands/network.rs"]
mod network;
use network::{InterfaceInfo, list_interfaces, mask_pair_token, pairing_hosts, pairing_uri};

const MAX_DEVICE_ID_BYTES: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CommandError {
    InvalidInput,
    NoPendingApproval,
    PersistenceFailure,
    InterfaceEnumerationFailed,
    ServerUnavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectionInfo {
    port: u16,
    effective_port: u16,
    interfaces: Vec<InterfaceInfo>,
    qrcode_uri: String,
    pair_code_masked: String,
    pc_id: String,
    name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrustedDeviceInfo {
    device_id: String,
    name: String,
    model: Option<String>,
    last_seen: u64,
    last_ip: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerHealth {
    state: &'static str,
    port: u16,
    effective_port: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HealthInfo {
    server: ServerHealth,
    stubs: Vec<&'static str>,
}

#[tauri::command]
pub(crate) fn get_connection_info(
    state: State<'_, AppState>,
) -> Result<ConnectionInfo, CommandError> {
    connection_info(&state)
}

fn connection_info(state: &AppState) -> Result<ConnectionInfo, CommandError> {
    ensure_available(state)?;
    let interfaces = list_interfaces()?;
    let hosts = pairing_hosts(&interfaces);

    let server_guard = lock_server(state);
    let server = server_guard
        .as_ref()
        .ok_or(CommandError::ServerUnavailable)?;
    let effective_port = server.local_port();
    let trust = server.trust();
    let store = trust.lock().unwrap_or_else(|poison| poison.into_inner());
    let pc_id = store.pc_id().to_owned();
    let token = store.pair_token().to_owned();
    let name = state.pc_name.clone();
    let qrcode_uri = pairing_uri(&hosts, effective_port, &token, &pc_id, &name);

    Ok(ConnectionInfo {
        port: state.requested_port,
        effective_port,
        interfaces,
        qrcode_uri,
        pair_code_masked: mask_pair_token(&token),
        pc_id,
        name,
    })
}

#[tauri::command]
pub(crate) fn regenerate_token(state: State<'_, AppState>) -> Result<String, CommandError> {
    ensure_available(&state)?;
    let server_guard = lock_server(&state);
    let server = server_guard
        .as_ref()
        .ok_or(CommandError::ServerUnavailable)?;
    let trust = server.trust();
    let mut store = trust.lock().unwrap_or_else(|poison| poison.into_inner());
    let token = store
        .regenerate_token()
        .map_err(|_| CommandError::PersistenceFailure)?;
    Ok(mask_pair_token(&token))
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn approve(
    state: State<'_, AppState>,
    request_id: u64,
    mode: String,
) -> Result<(), CommandError> {
    let decision = match mode.as_str() {
        "once" => Decision::ApproveOnce,
        "always" => Decision::ApproveAlways,
        _ => return Err(CommandError::InvalidInput),
    };
    decide_approval(&state, request_id, decision)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn reject(state: State<'_, AppState>, request_id: u64) -> Result<(), CommandError> {
    decide_approval(&state, request_id, Decision::Reject)
}

#[tauri::command]
pub(crate) fn list_trusted(
    state: State<'_, AppState>,
) -> Result<Vec<TrustedDeviceInfo>, CommandError> {
    ensure_available(&state)?;
    let server_guard = lock_server(&state);
    let server = server_guard
        .as_ref()
        .ok_or(CommandError::ServerUnavailable)?;
    let trust = server.trust();
    let store = trust.lock().unwrap_or_else(|poison| poison.into_inner());
    Ok(store
        .list()
        .into_iter()
        .map(|device| TrustedDeviceInfo {
            device_id: device.device_id,
            name: device.name,
            model: None,
            last_seen: device.last_seen,
            last_ip: device.last_ip,
        })
        .collect())
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn revoke_trusted(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<bool, CommandError> {
    validate_device_id(&device_id)?;
    ensure_available(&state)?;
    let server_guard = lock_server(&state);
    let server = server_guard
        .as_ref()
        .ok_or(CommandError::ServerUnavailable)?;
    let trust = server.trust();
    trust
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .revoke(&device_id)
        .map_err(|_| CommandError::PersistenceFailure)
}

#[tauri::command]
pub(crate) fn get_health(state: State<'_, AppState>) -> Result<HealthInfo, CommandError> {
    ensure_available(&state)?;
    let server_guard = lock_server(&state);
    let server = server_guard
        .as_ref()
        .ok_or(CommandError::ServerUnavailable)?;
    let effective_port = server.local_port();
    let state_name = if effective_port == state.requested_port {
        "listening"
    } else {
        "altPort"
    };
    Ok(HealthInfo {
        server: ServerHealth {
            state: state_name,
            port: state.requested_port,
            effective_port,
        },
        stubs: vec!["em breve"],
    })
}

fn decide_approval(
    state: &AppState,
    request_id: u64,
    decision: Decision,
) -> Result<(), CommandError> {
    ensure_available(state)?;
    validate_pending_request(state.pending_request_id.load(Ordering::Acquire), request_id)?;
    let server_guard = lock_server(state);
    let server = server_guard
        .as_ref()
        .ok_or(CommandError::ServerUnavailable)?;
    let decided = server.decide(request_id, decision);
    if !decided {
        return Err(CommandError::NoPendingApproval);
    }
    let _ = state.pending_request_id.compare_exchange(
        request_id,
        0,
        Ordering::AcqRel,
        Ordering::Acquire,
    );
    Ok(())
}

fn ensure_available(state: &AppState) -> Result<(), CommandError> {
    if state.shutting_down.load(Ordering::Acquire) {
        Err(CommandError::ServerUnavailable)
    } else {
        Ok(())
    }
}

fn validate_pending_request(pending_id: u64, request_id: u64) -> Result<(), CommandError> {
    if request_id == 0 {
        return Err(CommandError::InvalidInput);
    }
    if pending_id != request_id {
        return Err(CommandError::NoPendingApproval);
    }
    Ok(())
}

fn validate_device_id(device_id: &str) -> Result<(), CommandError> {
    if device_id.trim().is_empty()
        || device_id.len() > MAX_DEVICE_ID_BYTES
        || device_id.chars().any(char::is_control)
    {
        return Err(CommandError::InvalidInput);
    }
    Ok(())
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
