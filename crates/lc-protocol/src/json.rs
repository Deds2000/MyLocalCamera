//! Estruturas JSON do protocolo (camelCase).
//!
//! Campos desconhecidos são **ignorados** (sem `deny_unknown_fields`);
//! campos opcionais usam `#[serde(default)]`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub platform: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub android_api: u32,
}

/// Autenticação no HELLO: `{kind:"pair", token}` ou `{kind:"device", deviceId, key}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Auth {
    Pair {
        token: String,
    },
    Device {
        #[serde(rename = "deviceId")]
        device_id: String,
        key: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraMode {
    pub w: u32,
    pub h: u32,
    pub fps: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraCaps {
    pub id: String,
    pub facing: String,
    pub modes: Vec<CameraMode>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Caps {
    #[serde(default)]
    pub cameras: Vec<CameraCaps>,
    #[serde(default)]
    pub concurrent: bool,
    #[serde(default)]
    pub concurrent_modes: Vec<CameraMode>,
    #[serde(default)]
    pub audio: bool,
    #[serde(default)]
    pub codecs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hello {
    pub protocol: ProtocolVersion,
    pub app: AppInfo,
    pub device: DeviceInfo,
    pub auth: Auth,
    #[serde(default)]
    pub caps: Caps,
}

/// Motivo de recusa (WELCOME) / término (BYE); valor desconhecido vira `Other`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    BadToken,
    NotApproved,
    Revoked,
    Busy,
    VersionMismatch,
    BadAudioTicket,
    Timeout,
    ProtocolError,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PcInfo {
    pub id: String,
    pub name: String,
    pub app_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Welcome {
    pub accepted: bool,
    #[serde(default)]
    pub protocol: Option<ProtocolVersion>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub audio_ticket: Option<String>,
    #[serde(default)]
    pub device_key: Option<String>,
    #[serde(default)]
    pub pc: Option<PcInfo>,
    #[serde(default)]
    pub reason: Option<Reason>,
    #[serde(default)]
    pub min_supported: Option<ProtocolVersion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioHello {
    pub session_id: String,
    pub ticket: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    pub timeout_s: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTicket {
    pub ticket: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamConfig {
    pub stream: u8,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub rotation_hint: u16,
    pub facing: String,
    #[serde(default)]
    pub profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetItem {
    pub key: String,
    pub value: serde_json::Value,
}

/// SET aceita um item `{key,value}` ou uma lista deles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SetPayload {
    Many(Vec<SetItem>),
    One(SetItem),
}

/// Lados (traseira/frontal) para espelhamento e rotação.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sides<T> {
    pub back: T,
    pub front: T,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraState {
    pub mode: String,
    pub active: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoState {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Presentation {
    pub mirror: Sides<bool>,
    pub rotation: Sides<u16>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioState {
    pub enabled: bool,
    pub gain: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub rev: u64,
    #[serde(default)]
    pub camera: CameraState,
    #[serde(default)]
    pub video: VideoState,
    #[serde(default)]
    pub presentation: Presentation,
    #[serde(default)]
    pub audio: AudioState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub recv_fps: f32,
    pub dropped: u32,
    pub queue_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bye {
    pub reason: String,
}
