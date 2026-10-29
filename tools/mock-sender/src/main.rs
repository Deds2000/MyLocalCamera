//! Entrada do `mock-sender`: parse da CLI, logs e código de saída.

use clap::Parser;
use mock_sender::cli::Cli;
use mock_sender::session;

/// Inicializa logs com nível vindo de `RUST_LOG` (padrão `info`).
fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

#[tokio::main]
async fn main() {
    init_tracing();
    let cli = Cli::parse();
    let config = match cli.into_config() {
        Ok(config) => config,
        Err(message) => {
            eprintln!("erro: {message}");
            std::process::exit(1);
        }
    };

    match session::run(config).await {
        Ok(summary) => {
            println!(
                "vídeo: {} quadros / {} bytes · áudio: {} blocos · sessão {} · encerrada por {}",
                summary.video_frames,
                summary.video_bytes,
                summary.audio_blocks,
                summary.session_id,
                summary.closed_by.as_deref().unwrap_or("prazo/ctrl+c")
            );
        }
        Err(error) => {
            eprintln!("erro: {error}");
            std::process::exit(error.exit_code());
        }
    }
}
