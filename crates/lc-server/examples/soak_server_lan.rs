//! Servidor de *soak* para **LAN** (F2-T14): escuta em `0.0.0.0` para o
//! celular alcançar via Wi-Fi e imprime a URI de pareamento com o IP do PC
//! (detectado automaticamente). Variante de teste do `soak_server`
//! (que escuta só em `127.0.0.1`); o cartão F2-T14 pede exatamente essa
//! variante e o registro dela.
//!
//! Uso:
//! ```powershell
//! cargo run -p lc-server --example soak_server_lan            # aprova "sempre"
//! cargo run -p lc-server --example soak_server_lan -- --fresh # estado novo (token novo)
//! cargo run -p lc-server --example soak_server_lan -- --approve once
//! ```
//!
//! - `--fresh`: apaga o diretório de estado antes de subir (token novo —
//!   útil para repetir o fluxo de pareamento do zero);
//! - `--approve once`: aprova **uma** vez (reconectar depois → `not_approved`;
//!   FT-2.12). Padrão: `ApproveAlways` (IT-2.06 — reconecta sem novo diálogo).
//! - Em debug o `auto_approve` do config fica `false`: a decisão sai por
//!   aqui, via `Decision` (é uma ferramenta de teste, nunca o app).
//! - Windows: no primeiro uso o firewall pode perguntar — liberar na rede
//!   privada (registro: F2-T14).

use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::path::PathBuf;
use std::time::Duration;

use lc_server::config::ServerConfig;
use lc_server::events::{Decision, ServerEvent};
use lc_server::redact::short;
use lc_server::start;
use lc_server::stats::SessionStats;

/// IP local que o roteador usaria para tráfego externo (nenhum pacote é
/// enviado; o `connect` UDP só escolhe a interface).
fn lan_ip() -> IpAddr {
    let mut ip = IpAddr::from(Ipv4Addr::LOCALHOST);
    if let Ok(s) = UdpSocket::bind("0.0.0.0:0") {
        if s.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = s.local_addr() {
                ip = addr.ip();
            }
        }
    }
    ip
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    lc_testkit::init_tracing();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let fresh = args.iter().any(|a| a == "--fresh");
    let approve_once = match args.iter().position(|a| a == "--approve") {
        Some(i) => args.get(i + 1).is_some_and(|v| v == "once"),
        None => false,
    };
    let decision = if approve_once {
        Decision::ApproveOnce
    } else {
        Decision::ApproveAlways
    };

    let dir: PathBuf = std::env::temp_dir().join("localcamera-soak-lan");
    if fresh {
        let _ = std::fs::remove_dir_all(&dir);
    }
    let mut cfg = ServerConfig::for_tests(dir, false);
    cfg.bind_ip = IpAddr::from(Ipv4Addr::UNSPECIFIED); // 0.0.0.0 — LAN
    cfg.port = 48650;
    cfg.pc_name = "Soak LAN".to_owned();
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
    let ip = lan_ip();
    println!("bind: 0.0.0.0:{port} (IP do PC: {ip})");
    println!(
        "uri: localcamera://connect?v=1&hosts={ip}&port={port}&token={token}&pc={pc_id}&name=Soak%20LAN"
    );
    println!(
        "aprovar: {}",
        if approve_once { "uma vez" } else { "sempre" }
    );
    println!("pronto; Ctrl+C encerra");

    let mut ticker = tokio::time::interval(Duration::from_secs(10));
    let mut last: Option<(String, SessionStats)> = None;
    loop {
        let mut approve: Option<u64> = None;
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
            handle.decide(request_id, decision);
        }
    }

    handle.shutdown().await;
    Ok(())
}
