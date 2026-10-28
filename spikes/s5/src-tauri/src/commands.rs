use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::AppHandle;
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
    /// `Some(path)` quando iniciado com `--json` (modo auto completo).
    pub json: Option<String>,
}

#[tauri::command]
pub fn auto_config(state: State<AppState>) -> AutoConfig {
    AutoConfig {
        mode: state.args.mode.map(|c| c.to_string()),
        seconds: state.args.seconds,
        json: state.args.json.clone(),
    }
}

/// Fim da medição em modo auto: grava o resumo JSON no caminho de `--json`
/// e encerra o app. Chamado uma única vez pelo frontend.
#[tauri::command]
pub fn auto_finish(
    app: AppHandle,
    state: State<AppState>,
    summary: String,
) -> Result<(), String> {
    let path = match state.args.json.clone() {
        Some(p) => p,
        None => return Ok(()), // sem --json: nada a fazer (fluxo de botões)
    };
    let pb = std::path::Path::new(&path);
    if let Some(dir) = pb.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(pb, summary).map_err(|e| format!("falha ao escrever {path}: {e}"))?;
    app.exit(0);
    Ok(())
}
