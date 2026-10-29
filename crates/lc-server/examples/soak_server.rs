//! Servidor de *soak* (F1-T13): sobe em `127.0.0.1:48650`, imprime a URI
//! de pareamento e as estatísticas a cada 10 s; roda até Ctrl+C.
//!
//! Em release o `auto_approve` é ignorado (F1-T06), então o exemplo
//! também aprova pedidos vindos de `ApprovalRequested` — é uma ferramenta
//! de teste, nunca o app.

use std::time::Duration;

use lc_server::config::ServerConfig;
use lc_server::events::{Decision, ServerEvent};
use lc_server::redact::short;
use lc_server::start;
use lc_server::stats::SessionStats;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    lc_testkit::init_tracing();

    let dir = std::env::temp_dir().join("localcamera-soak");
    let mut cfg = ServerConfig::for_tests(dir, true);
    cfg.port = 48650;
    cfg.pc_name = "Soak PC".to_owned();
    cfg.app_version = env!("CARGO_PKG_VERSION").to_owned();
    cfg.hello_timeout = Duration::from_secs(5);
    cfg.ping_interval = Duration::from_secs(2);
    cfg.liveness_timeout = Duration::from_secs(6);
    cfg.conn_rate_max = 1_000;

    let mut handle = start(cfg).await?;
    let port = handle.local_port();
    let (token, pc_id) = {
        let trust = handle.trust();
        let guard = trust.lock().unwrap_or_else(|p| p.into_inner());
        (guard.pair_token().to_owned(), guard.pc_id().to_owned())
    };
    println!("porta: {port}");
    println!(
        "uri: localcamera://connect?v=1&hosts=127.0.0.1&port={port}&token={token}&pc={pc_id}&name=Soak%20PC"
    );
    println!("pronto; Ctrl+C encerra");

    let mut ticker = tokio::time::interval(Duration::from_secs(10));
    let mut last: Option<(String, SessionStats)> = None;
    loop {
        let mut approve = None;
        tokio::select! {
            _ = ticker.tick() => match &last {
                Some((session_id, stats)) => println!(
                    "stats session={} fps={:.1} kbps={:.0} audio/s={:.1} rtt={:?} frames={} blocks={}",
                    short(session_id),
                    stats.video_fps[0],
                    stats.video_kbps[0],
                    stats.audio_blocks_per_s,
                    stats.rtt_ms,
                    stats.totals.video_frames[0],
                    stats.totals.audio_blocks,
                ),
                None => println!("sem sessao ativa"),
            },
            event = handle.events().recv() => match event {
                Some(ServerEvent::ApprovalRequested(request)) => {
                    println!("aprovando dispositivo {}", request.device_id);
                    approve = Some(request.request_id);
                }
                Some(ServerEvent::SessionStarted { device_name, .. }) => {
                    println!("sessao iniciada: {device_name}");
                }
                Some(ServerEvent::SessionEnded { reason, .. }) => {
                    println!("sessao encerrada: {reason:?}");
                    last = None;
                }
                Some(ServerEvent::Stats { session_id, stats }) => {
                    last = Some((session_id, stats));
                }
                Some(_) => {}
                None => break,
            },
            _ = tokio::signal::ctrl_c() => break,
        }
        if let Some(request_id) = approve {
            handle.decide(request_id, Decision::ApproveAlways);
        }
    }

    handle.shutdown().await;
    Ok(())
}
