mod window;

use tauri::{Manager, WindowEvent};

use crate::window as window_state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Instância única: a 2ª execução foca a janela existente (F3-T01).
        // Registrado antes dos demais plugins, por convenção do plugin.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Restaura tamanho/posição/estado salvos (validando o monitor).
            let window = app
                .get_webview_window("main")
                .expect("janela main declarada no tauri.conf.json");
            let data_dir = app
                .handle()
                .path()
                .app_data_dir()
                .expect("app data dir do Tauri");
            let monitors = window_state::monitors_logical(&window.available_monitors()?);
            if let Some(state) = window_state::resolve(window_state::load(&data_dir), &monitors) {
                let _ = window.set_position(tauri::LogicalPosition::new(state.x, state.y));
                let _ = window.set_size(tauri::LogicalSize::new(state.w as f64, state.h as f64));
                if state.maximized {
                    let _ = window.maximize();
                }
            }

            // Salva o estado em CloseRequested e em maximize/restore (Resized).
            let saved_window = window.clone();
            window.on_window_event(move |event| match event {
                WindowEvent::CloseRequested { .. } | WindowEvent::Resized(_) => {
                    window_state::save_current(&saved_window, &data_dir);
                }
                _ => {}
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
