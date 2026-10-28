mod commands;
mod feed;
mod meminfo;

pub fn run() {
    tauri::Builder::default()
        .manage(commands::AppState {
            args: feed::parse_args(),
            feed: std::sync::Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_mode_a,
            commands::start_mode_b,
            commands::stop_feed,
            commands::process_info,
            commands::csv_append,
            commands::auto_config,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o app Tauri (S5 spike)");
}
