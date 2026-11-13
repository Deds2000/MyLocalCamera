//! Persistência e validação do tamanho/posição da janela (F3-T01).
//!
//! O estado vive em `<app_data_dir>/window.json`. Arquivo ausente ou corrompido
//! nunca derruba o app: `load` devolve `None` e o chamador usa o padrão
//! 1280×800 centralizado (FT-3.09). A posição salva só é aceita se intersecta
//! um monitor de `available_monitors()`; caso contrário centralizamos no 1º
//! monitor.
//!
//! Os limites espelham `tauri.conf.json` (min 960×640; padrão 1280×800).
//!
//! Nota: `tauri::Monitor` não tem construtor público, então as funções puras
//! trabalham com [`MonitorRect`] (pixels lógicos) e a conversão acontece em
//! [`monitors_logical`].

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{Monitor, PhysicalPosition};

/// Padrão da janela (tauri.conf.json).
pub const DEFAULT_W: u32 = 1280;
pub const DEFAULT_H: u32 = 800;
/// Mínimo da janela (tauri.conf.json).
pub const MIN_W: u32 = 960;
pub const MIN_H: u32 = 640;

const FILE_NAME: &str = "window.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub w: u32,
    pub h: u32,
    pub x: i32,
    pub y: i32,
    pub maximized: bool,
}

impl WindowState {
    pub fn default_state() -> Self {
        Self {
            w: DEFAULT_W,
            h: DEFAULT_H,
            x: 0,
            y: 0,
            maximized: false,
        }
    }
}

/// Retângulo de um monitor em pixels lógicos (para as funções puras).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorRect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

/// Converte os monitores do Tauri para retângulos lógicos.
pub fn monitors_logical(monitors: &[Monitor]) -> Vec<MonitorRect> {
    monitors
        .iter()
        .map(|mon| {
            let scale = mon.scale_factor().max(0.1);
            let size = mon.size();
            let position = mon.position();
            MonitorRect {
                x: (position.x as f64 / scale).round() as i32,
                y: (position.y as f64 / scale).round() as i32,
                w: (size.width as f64 / scale).round() as u32,
                h: (size.height as f64 / scale).round() as u32,
            }
        })
        .collect()
}

pub fn file_path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

/// Lê o estado salvo. `None` se o arquivo não existe, está corrompido ou tem
/// tamanho abaixo do mínimo (sem panic — FT-3.09).
pub fn load(data_dir: &Path) -> Option<WindowState> {
    let raw = fs::read_to_string(file_path(data_dir)).ok()?;
    let state: WindowState = serde_json::from_str(&raw).ok()?;
    if state.w < MIN_W || state.h < MIN_H {
        return None;
    }
    Some(state)
}

/// Escreve o estado. Falhas de disco são ignoradas (o app segue rodando).
pub fn save(data_dir: &Path, state: &WindowState) {
    let path = file_path(data_dir);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(state) {
        let _ = fs::write(path, json);
    }
}

/// Verifica se a geometria salva (pixels lógicos) intersecta ao menos um
/// monitor.
pub fn valid_on_monitors(state: &WindowState, monitors: &[MonitorRect]) -> bool {
    monitors.iter().any(|mon| {
        state.x < (mon.x + mon.w as i32)
            && state.x + state.w as i32 > mon.x
            && state.y < (mon.y + mon.h as i32)
            && state.y + state.h as i32 > mon.y
    })
}

/// Grava a geometria atual da janela em pixels lógicos.
///
/// Quando a janela está maximizada, mantém a última geometria restaurada salva
/// (o tamanho/posição do estado maximizado não serve para restaurar). Falhas de
/// leitura/escrita não derrubam o app.
pub fn save_current(window: &tauri::WebviewWindow, data_dir: &Path) {
    let scale = window.scale_factor().unwrap_or(1.0).max(0.1);
    let (w, h, x, y) = match window.inner_size() {
        Ok(size) => {
            let pos = window
                .inner_position()
                .unwrap_or_else(|_| PhysicalPosition::new(0, 0));
            (
                (size.width as f64 / scale).round() as u32,
                (size.height as f64 / scale).round() as u32,
                (pos.x as f64 / scale).round() as i32,
                (pos.y as f64 / scale).round() as i32,
            )
        }
        Err(_) => return,
    };
    let maximized = window.is_maximized().unwrap_or(false);
    let state = if maximized {
        match load(data_dir) {
            Some(prev) => WindowState {
                w: prev.w,
                h: prev.h,
                x: prev.x,
                y: prev.y,
                maximized: true,
            },
            None => WindowState {
                w,
                h,
                x,
                y,
                maximized: true,
            },
        }
    } else {
        WindowState {
            w,
            h,
            x,
            y,
            maximized: false,
        }
    };
    save(data_dir, &state);
}

/// Decide o estado a aplicar na abertura.
///
/// - estado salvo válido → restaura exatamente o salvo;
/// - ausente/corrompido/fora de todos os monitores → padrão 1280×800
///   centralizado no 1º monitor (UT-3.14/FT-3.09);
/// - nenhum monitor (não deve ocorrer) → `None` (mantém o padrão do conf).
pub fn resolve(saved: Option<WindowState>, monitors: &[MonitorRect]) -> Option<WindowState> {
    if let Some(state) = saved {
        if valid_on_monitors(&state, monitors) {
            return Some(state);
        }
    }
    let base = monitors.first()?;
    let state = WindowState::default_state();
    Some(WindowState {
        x: base.x + ((base.w as i32 - state.w as i32) / 2),
        y: base.y + ((base.h as i32 - state.h as i32) / 2),
        ..state
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(x: i32, y: i32, w: u32, h: u32) -> MonitorRect {
        MonitorRect { x, y, w, h }
    }

    fn tmp_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("lc-f3t01-window-{}-{}", std::process::id(), tag))
    }

    fn clean(dir: &Path) {
        let _ = fs::remove_file(file_path(dir));
        let _ = fs::remove_dir(dir);
    }

    // UT-3.14 — restauração da janela
    #[test]
    fn restora_estado_valido_salvo() {
        let mons = vec![monitor(0, 0, 1920, 1080)];
        let saved = WindowState {
            w: 1100,
            h: 700,
            x: 100,
            y: 120,
            maximized: false,
        };
        let got = resolve(Some(saved), &mons).expect("deve restaurar");
        assert_eq!(got, saved);
    }

    #[test]
    fn estado_fora_de_todos_os_monitors_centraliza_no_primeiro() {
        let mons = vec![monitor(0, 0, 1920, 1080), monitor(1920, 0, 1920, 1080)];
        let saved = WindowState {
            w: 1280,
            h: 800,
            x: 10_000, // fora de todos os monitores
            y: 10_000,
            maximized: false,
        };
        let got = resolve(Some(saved), &mons).expect("deve centralizar");
        assert_eq!((got.w, got.h), (1280, 800));
        assert_eq!((got.x, got.y), (320, 140)); // centro do 1º monitor
        assert!(!got.maximized);
    }

    #[test]
    fn ausente_centraliza_no_primeiro_monitor() {
        let mons = vec![monitor(640, 200, 2560, 1440)];
        let got = resolve(None, &mons).expect("deve centralizar");
        assert_eq!(
            (got.x, got.y),
            (640 + (2560 - 1280) / 2, 200 + (1440 - 800) / 2)
        );
        assert_eq!((got.w, got.h), (1280, 800));
    }

    // FT-3.09 — window.json corrompido
    #[test]
    fn json_corrompido_devolve_none_sem_panic() {
        let dir = tmp_dir("corrompido");
        let _ = fs::create_dir_all(&dir);
        fs::write(file_path(&dir), "{isso não é JSON válido").expect("grava fixture");
        assert_eq!(load(&dir), None);
        clean(&dir);
    }

    #[test]
    fn arquivo_ausente_devolve_none() {
        let dir = tmp_dir("ausente");
        assert_eq!(load(&dir), None);
        clean(&dir);
    }

    #[test]
    fn tamanho_abaixo_do_minimo_devolve_none() {
        let dir = tmp_dir("min");
        let _ = fs::create_dir_all(&dir);
        let small = WindowState {
            w: 100,
            h: 800,
            x: 0,
            y: 0,
            maximized: false,
        };
        fs::write(file_path(&dir), serde_json::to_string(&small).unwrap()).expect("grava fixture");
        assert_eq!(load(&dir), None);
        clean(&dir);
    }

    #[test]
    fn save_e_load_fazem_roundtrip() {
        let dir = tmp_dir("roundtrip");
        let state = WindowState {
            w: 1000,
            h: 700,
            x: -50,
            y: 30,
            maximized: true,
        };
        save(&dir, &state);
        assert_eq!(load(&dir), Some(state));
        clean(&dir);
    }

    #[test]
    fn valid_on_monitors_exige_intersecao() {
        let mons = vec![monitor(0, 0, 1920, 1080)];
        let dentro = WindowState {
            w: 1280,
            h: 800,
            x: 100,
            y: 100,
            maximized: false,
        };
        let fora = WindowState {
            w: 1280,
            h: 800,
            x: 5000,
            y: 5000,
            maximized: false,
        };
        let canto = WindowState {
            w: 1280,
            h: 800,
            x: 1900, // toca o monitor em 20 px
            y: 100,
            maximized: false,
        };
        assert!(valid_on_monitors(&dentro, &mons));
        assert!(!valid_on_monitors(&fora, &mons));
        assert!(valid_on_monitors(&canto, &mons));
    }
}
