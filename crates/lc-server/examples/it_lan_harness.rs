//! Servidor de teste **LAN** para IT-2.05/IT-2.06 (F2-T14) — **seguro com
//! segredos**.
//!
//! Variante segura do `soak_server_lan` (esse exemplo imprime a URI completa
//! de pareamento, com token, no stdout — **não** usá-lo). Diferenças:
//!
//! - estado em diretório novo e único por execução:
//!   `%TEMP%\localcamera-it-<pid>-<ts>` (nunca reutiliza nem apaga estado
//!   alheio);
//! - bind no IP de LAN detectado (não em `0.0.0.0`) e porta efêmera
//!   (`port = 0`), salvo `--port`/`--all`;
//! - token/URI de pareamento ficam só em memória; um broker TCP HTTP de uso
//!   único os entrega ao teste instrumentado do Android, sem arquivo, stdout,
//!   argv ou log;
//! - stdout contém só fatos não sensíveis: caminhos, porta, `pcId`, stats;
//! - watchdog embutido: `--seconds N` encerra sozinho;
//! - cleanup: o diretório é removido **somente** se esta execução o criou
//!   (`--keep` mantém; `--state <dir>` reaproveita estado existente e nunca
//!   apaga); o broker é encerrado com o harness.
//!
//! Uso (PowerShell, raiz do repo; **apenas com autorização explícita** —
//! ver `docs/tasks/fase-2/F2-T14-harness-lan.md`):
//! ```powershell
//! cargo run -p lc-server --example it_lan_harness -- --seconds 300
//! cargo run -p lc-server --example it_lan_harness -- --seconds 300 --keep            # IT-2.06 fase 1
//! cargo run -p lc-server --example it_lan_harness -- --state <dir> --port 48651 --seconds 300  # fase 2
//! ```
//! Decisão de aprovação (padrão "sempre"): `--approve once|always|reject`.

use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use lc_server::config::ServerConfig;
use lc_server::events::{ApprovalOutcome, Decision, ServerEvent};
use lc_server::redact::short;
use lc_server::stats::SessionStats;
use lc_server::{start, ServerHandle};

#[path = "it_lan_harness/pairing_broker.rs"]
mod pairing_broker;

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

struct Opts {
    state_dir: Option<PathBuf>,
    approve: Decision,
    seconds: Option<u64>,
    port: u16,
    bind_all: bool,
    keep: bool,
}

fn parse_args() -> Result<Opts, String> {
    let mut o = Opts {
        state_dir: None,
        approve: Decision::ApproveAlways,
        seconds: None,
        port: 0,
        bind_all: false,
        keep: false,
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--state" => {
                i += 1;
                o.state_dir = Some(PathBuf::from(arg(&args, i, "--state")?));
            }
            "--port" => {
                i += 1;
                o.port = arg(&args, i, "--port")?
                    .parse()
                    .map_err(|_| "--port: número inválido".to_owned())?;
            }
            "--seconds" => {
                i += 1;
                o.seconds = Some(
                    arg(&args, i, "--seconds")?
                        .parse()
                        .map_err(|_| "--seconds: número inválido".to_owned())?,
                );
            }
            "--approve" => {
                i += 1;
                o.approve = match arg(&args, i, "--approve")?.as_str() {
                    "always" => Decision::ApproveAlways,
                    "once" => Decision::ApproveOnce,
                    "reject" => Decision::Reject,
                    other => {
                        return Err(format!(
                            "--approve: valor inválido '{other}' (use always|once|reject)"
                        ))
                    }
                }
            }
            "--all" => o.bind_all = true,
            "--keep" => o.keep = true,
            other => return Err(format!("flag desconhecida: {other}")),
        }
        i += 1;
    }
    Ok(o)
}

fn arg(args: &[String], i: usize, flag: &str) -> Result<String, String> {
    args.get(i)
        .cloned()
        .ok_or_else(|| format!("{flag} precisa de um valor"))
}

fn decision_label(d: Decision) -> &'static str {
    match d {
        Decision::ApproveAlways => "sempre",
        Decision::ApproveOnce => "uma vez",
        Decision::Reject => "recusar",
    }
}

fn outcome_label(o: &ApprovalOutcome) -> String {
    match o {
        ApprovalOutcome::Approved(d) => format!("aprovado ({})", decision_label(*d)),
        ApprovalOutcome::Rejected => "recusado".to_owned(),
        ApprovalOutcome::TimedOut => "timeout".to_owned(),
        ApprovalOutcome::Cancelled => "cancelado (celular desconectou)".to_owned(),
    }
}

#[tokio::main]
async fn main() {
    lc_testkit::init_tracing();
    if let Err(e) = run().await {
        eprintln!("erro: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let opts = parse_args()?;

    // 1) Estado isolado: só o que esta execução cria (o cleanup é limitado a ele).
    let (dir, created) = if let Some(d) = opts.state_dir {
        if !d.exists() {
            return Err(format!("--state: diretório não existe: {}", d.display()).into());
        }
        (d, false)
    } else {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|t| t.as_secs())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("localcamera-it-{}-{ts}", std::process::id()));
        if dir.exists() {
            return Err(format!("colisão com diretório existente: {}", dir.display()).into());
        }
        (dir, true)
    };

    // 2) Bind privado: o IP de LAN detectado (`--all` amarra 0.0.0.0).
    let ip = lan_ip();
    let bind_ip = if opts.bind_all {
        IpAddr::from(Ipv4Addr::UNSPECIFIED)
    } else {
        ip
    };
    if !opts.bind_all && ip.is_loopback() {
        println!(
            "aviso: IP de LAN não detectado (fallback 127.0.0.1); confira `ipconfig` ou use --all"
        );
    }

    let mut cfg = ServerConfig::for_tests(dir.clone(), false);
    cfg.bind_ip = bind_ip;
    cfg.port = opts.port;
    cfg.pc_name = "IT LAN".to_owned();
    cfg.app_version = env!("CARGO_PKG_VERSION").to_owned();
    cfg.hello_timeout = Duration::from_secs(5);
    cfg.ping_interval = Duration::from_secs(2);
    cfg.liveness_timeout = Duration::from_secs(6);
    cfg.conn_rate_max = 1_000;

    let mut handle = start(cfg).await?;

    // 3) URI e token só em memória; o broker faz uma entrega HTTP de uso único.
    let port = handle.local_port();
    let (token, pc_id) = {
        let trust = handle.trust();
        let guard = trust.lock().unwrap_or_else(|p| p.into_inner());
        (guard.pair_token().to_owned(), guard.pc_id().to_owned())
    };
    let host = if opts.bind_all { ip } else { bind_ip };
    let uri = format!(
        "localcamera://connect?v=1&hosts={host}&port={port}&token={token}&pc={pc_id}&name=IT%20LAN"
    );
    let broker = pairing_broker::PairingBroker::start(bind_ip, uri).await?;
    let broker_host = if opts.bind_all { ip } else { bind_ip };

    println!("estado: {}", dir.display());
    println!("bind: {bind_ip}:{port}");
    println!("pc: {pc_id}");
    println!(
        "broker: http://{}:{}/pairing (sem token; use o teste instrumentado)",
        broker_host,
        broker.address().port()
    );
    println!("aprovar: {}", decision_label(opts.approve));
    if let Some(s) = opts.seconds {
        println!("watchdog: encerra sozinho em {s} s");
    }
    println!("pronto; Ctrl+C encerra");

    // 4) Laço de eventos (fatos não sensíveis) + watchdog embutido opcional.
    let timed_out = match opts.seconds {
        Some(s) => {
            let fut = events_loop(&mut handle, opts.approve);
            tokio::time::timeout(Duration::from_secs(s), fut)
                .await
                .is_err()
        }
        None => {
            events_loop(&mut handle, opts.approve).await;
            false
        }
    };
    if timed_out {
        println!("watchdog: tempo esgotado; encerrando");
    }

    broker.shutdown().await;
    handle.shutdown().await;

    // 5) Cleanup: somente o diretório criado por esta execução.
    if created && !opts.keep {
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => println!("estado: removido (criado por esta execução)"),
            Err(e) => println!(
                "aviso: falha ao remover {} ({e}); remova manualmente",
                dir.display()
            ),
        }
    } else {
        println!("estado: mantido {}", dir.display());
    }
    Ok(())
}

/// Laço de eventos: aprovação por `Decision` (ferramenta de teste — a UI do
/// app nunca participa), stats a cada 10 s, nada sensível.
async fn events_loop(handle: &mut ServerHandle, decision: Decision) {
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
                Some(ServerEvent::PortChanged { requested, actual }) => {
                    println!("aviso: porta {requested} ocupada; servidor usando {actual}")
                }
                Some(ServerEvent::ApprovalRequested(request)) => {
                    println!("pareamento: {} ({})", request.device_id, request.name);
                    approve = Some(request.request_id);
                }
                Some(ServerEvent::ApprovalResolved { outcome, .. }) => {
                    println!("aprovacao: {}", outcome_label(&outcome))
                }
                Some(ServerEvent::SessionStarted { device_name, peer, .. }) => {
                    println!("sessao iniciada: {} de {}", device_name, peer)
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
}
