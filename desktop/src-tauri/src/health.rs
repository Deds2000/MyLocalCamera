use serde::Serialize;

use crate::server::{AppState, lock_server};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerHealth {
    pub state: &'static str,
    pub port: u16,
    pub effective_port: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthInfo {
    pub server: ServerHealth,
    pub stubs: Vec<&'static str>,
}

impl HealthInfo {
    pub fn from_state(state: &AppState) -> Self {
        let effective_port = lock_server(state)
            .as_ref()
            .map_or(0, |server| server.local_port());
        let state_name = if effective_port == 0 {
            "failed"
        } else if effective_port == state.requested_port {
            "listening"
        } else {
            "altPort"
        };
        Self {
            server: ServerHealth {
                state: state_name,
                port: state.requested_port,
                effective_port,
            },
            stubs: vec!["em breve"],
        }
    }
}
