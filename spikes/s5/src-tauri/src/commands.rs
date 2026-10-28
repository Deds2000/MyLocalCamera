use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;

use crate::feed::{self, ParseArgs, StopFlag};

pub struct AppState {
    pub args: ParseArgs,
    pub feed: Mutex<Option<Arc<StopFlag>>>,
}

#[tauri::command]
pub fn start_mode_a(
    state: State<AppState>,
    channel: Channel<Vec<u8>>,
) -> Result<String, String> {
    start(&state, feed::spawn_mode_a, channel)
}

#[tauri::command]
pub fn start_mode_b(
    state: State<AppState>,
    channel: Channel<Vec<u8>>,
) -> Result<String, String> {
    start(&state, feed::spawn_mode_b, channel)
}

fn start(
    state: &State<AppState>,
    spawn: fn(Arc<StopFlag>, Channel<Vec<u8>>),
    channel: Channel<Vec<u8>>,
) -> Result<String, String> {
    let mut g = state.feed.lock().map_err(|e| e.to_string())?;
    if let Some(old) = g.take() {
        old.stop();
    }
    let flag = Arc::new(StopFlag::new());
    g.replace(flag.clone());
    spawn(flag, channel);
    Ok("started".into())
}

#[tauri::command]
pub fn stop_feed(state: State<AppState>) -> Result<(), String> {
    if let Some(flag) = state.feed.lock().map_err(|e| e.to_string())?.take() {
        flag.stop();
    }
    Ok(())
}

#[derive(Serialize)]
pub struct ProcessInfo {
    pub mem_mb: u64,
    pub pid: u32,
}

#[tauri::command]
pub fn process_info() -> ProcessInfo {
    ProcessInfo {
        mem_mb: crate::meminfo::working_set_mb(),
        pid: std::process::id(),
    }
}

/// Apende uma linha em `spikes/s5/medicoes.csv` (cria com cabeçalho se nova).
/// O caminho é relativo ao diretório de trabalho do processo (raiz do repo).
#[tauri::command]
pub fn csv_append(line: String, header: String) -> Result<(), String> {
    use std::io::Write;
    let dir = std::env::current_dir().map_err(|e| e.to_string())?;
    let path = dir.join("spikes/s5/medicoes.csv");
    let is_new = !path.exists();
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    if is_new {
        writeln!(f, "{header}").map_err(|e| e.to_string())?;
    }
    writeln!(f, "{line}").map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct AutoConfig {
    pub mode: Option<String>,
    pub seconds: u32,
}

#[tauri::command]
pub fn auto_config(state: State<AppState>) -> AutoConfig {
    AutoConfig {
        mode: state.args.mode.map(|c| c.to_string()),
        seconds: state.args.seconds,
    }
}
