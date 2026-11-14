mod commands;
mod events_pump;
mod server;
mod window;

use std::sync::atomic::Ordering;

use lc_server::events::Decision;
use tauri::{Manager, RunEvent, WindowEvent};

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
        .invoke_handler(tauri::generate_handler![
            commands::get_connection_info,
            commands::regenerate_token,
            commands::approve,
            commands::reject,
            commands::list_trusted,
            commands::revoke_trusted,
            commands::get_health,
        ])
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

            let server = tauri::async_runtime::block_on(server::AppState::start(data_dir.clone()))
                .map_err(Box::<dyn std::error::Error>::from)?;
            app.manage(server);
            events_pump::spawn(app.handle().clone());

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
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                let state = app.state::<server::AppState>();
                state.shutting_down.store(true, Ordering::Release);
                tauri::async_runtime::block_on(async {
                    let pending_id = state.pending_request_id.swap(0, Ordering::AcqRel);
                    if let Some(mut server) = server::take_server(&state) {
                        if pending_id != 0 {
                            let _ = server.decide(pending_id, Decision::Reject);
                        }
                        server.shutdown().await;
                    }
                });
            }
        });
}
