//! Envio de quadros por `tauri::ipc::Channel` a 30 fps (loop do fixture).
//! Wire: [1 byte flags (bit0=keyframe)] [8 bytes u64 LE unix-ms] [payload].

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use lc_testkit::annexb;
use tauri::ipc::Channel;

pub struct ParseArgs {
    pub mode: Option<char>,
    pub seconds: u32,
    /// Em modo auto com `--json`, o app escreve o resumo final neste caminho e sai.
    pub json: Option<String>,
}

pub fn parse_args() -> ParseArgs {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut mode: Option<char> = None;
    let mut seconds: u32 = 60;
    let mut json: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args.get(i).map(|s| s.as_str()) {
            // `--auto` = modo automatizado; `--mode` mantido p/ compatibilidade.
            Some("--auto") | Some("--mode") => {
                if let Some(v) = args.get(i + 1) {
                    mode = v.chars().next();
                    i += 1;
                }
            }
            Some("--seconds") => {
                if let Some(v) = args.get(i + 1) {
                    seconds = v.parse().unwrap_or(60);
                    i += 1;
                }
            }
            Some("--json") => {
                if let Some(v) = args.get(i + 1) {
                    json = Some(v.clone());
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    ParseArgs { mode, seconds, json }
}

pub struct StopFlag(AtomicBool);

impl StopFlag {
    pub fn new() -> Self {
        Self(AtomicBool::new(false))
    }
    pub fn stop(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    pub fn is_set(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

pub fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn pack(key: bool, ts: u64, payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(9 + payload.len());
    v.push(if key { 1 } else { 0 });
    v.extend_from_slice(&ts.to_le_bytes());
    v.extend_from_slice(payload);
    v
}

fn fixture_path(rel: &str) -> Result<PathBuf, String> {
    Ok(std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join(rel))
}

/// Pacing a 30 fps com correção de drift; para no flag ou channel fechado.
fn send_loop(
    flag: Arc<StopFlag>,
    ch: Channel<Vec<u8>>,
    mut next: impl FnMut(usize) -> Vec<u8>,
) {
    let period_ns: u128 = 33_333_333;
    let start = Instant::now();
    let mut i = 0usize;
    loop {
        if flag.is_set() {
            return;
        }
        let due = (i as u128) * period_ns;
        let now = start.elapsed().as_nanos();
        if now < due {
            std::thread::sleep(Duration::from_nanos((due - now) as u64));
        }
        let msg = next(i);
        if ch.send(msg).is_err() {
            return; // webview fechou / channel descartado
        }
        i += 1;
    }
}

/// Modo A: lê `fixtures/hd30.h264`, divide em AUs e envia cada um a 30 fps.
pub fn spawn_mode_a(flag: Arc<StopFlag>, ch: Channel<Vec<u8>>) {
    std::thread::spawn(move || {
        let path = match fixture_path("fixtures/hd30.h264") {
            Ok(p) => p,
            Err(e) => {
                eprintln!("modo A: {e}");
                return;
            }
        };
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("modo A: falha ao ler {}: {e}", path.display());
                return;
            }
        };
        let aus: Vec<(bool, Vec<u8>)> =
            annexb::split_access_units(&data)
                .into_iter()
                .map(|r| {
                    // rustc 1.99: Range não é mais Copy
                    let au = &data[r.clone()];
                    (annexb::is_idr(au), au.to_vec())
                })
                .collect();
        eprintln!("modo A: {} AUs, {} bytes", aus.len(), data.len());
        let n = aus.len();
        send_loop(flag, ch, move |i| {
            let (key, bytes) = &aus[i % n];
            pack(*key, unix_ms(), bytes)
        });
    });
}

/// Modo B: envia cada JPEG de `fixtures/hd30-960/` (ordenado) a 30 fps.
pub fn spawn_mode_b(flag: Arc<StopFlag>, ch: Channel<Vec<u8>>) {
    std::thread::spawn(move || {
        let dir = match fixture_path("fixtures/hd30-960") {
            Ok(d) => d,
            Err(e) => {
                eprintln!("modo B: {e}");
                return;
            }
        };
        let mut files: Vec<PathBuf> = match std::fs::read_dir(&dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "jpg"))
                .collect(),
            Err(e) => {
                eprintln!("modo B: falha ao ler {}: {e}", dir.display());
                return;
            }
        };
        files.sort();
        let mut frames: Vec<Vec<u8>> = Vec::with_capacity(files.len());
        for p in &files {
            match std::fs::read(p) {
                Ok(b) => frames.push(b),
                Err(e) => eprintln!("modo B: falha ao ler {}: {e}", p.display()),
            }
        }
        if frames.is_empty() {
            eprintln!("modo B: nenhum JPEG encontrado");
            return;
        }
        eprintln!("modo B: {} JPEGs", frames.len());
        send_loop(flag, ch, move |i| {
            let bytes = &frames[i % frames.len()];
            pack(false, unix_ms(), bytes)
        });
    });
}
