mod commands;
mod feed;
mod meminfo;

use tauri::Manager;

pub fn run() {
    let args = feed::parse_args();
    tauri::Builder::default()
        .manage(commands::AppState {
            args,
            feed: std::sync::Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_mode_a,
            commands::start_mode_b,
            commands::stop_feed,
            commands::process_info,
            commands::csv_append,
            commands::auto_config,
            commands::auto_finish,
        ])
        // Modo auto com `--json`: trava de segurança — se o frontend não
        // chamar `auto_finish`, o app sai sozinho (exit code 2).
        .setup(|app| {
            let st = app.state::<commands::AppState>();
            if st.args.json.is_some() {
                let secs = st.args.seconds.saturating_add(30) as u64;
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(secs));
                    handle.exit(2);
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o app Tauri (S5 spike)");
}
