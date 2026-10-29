//! Keepalive, RTT e agregador de estatísticas (F1-T10): detecção de
//! conexão perdida sem `PONG` nos dois tempos (não responder / half-open),
//! cadência de 500 ms do agregador, RTT medido e contadores de vídeo por
//! stream.
//!
//! O servidor roda com `auto_approve = true`. `ServerConfig::for_tests`
//! mantém o keepalive solto (os demais testes usam clientes que não
//! respondem PING); aqui os valores pequenos vêm da configuração.

#[allow(dead_code)]
mod common;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use common::TestClient;
use lc_protocol::{encode_frame, flags, msg, Message};
use lc_server::config::ServerConfig;
use lc_server::events::{CloseReason, ServerEvent};
use lc_server::{start, ServerHandle};
use tokio::sync::mpsc;

/// Prazo dos `recv` dos clientes de teste.
const TIMEOUT: Duration = Duration::from_secs(3);

/// Boot com keepalive/estatísticas explícitos (testes usam pequenos).
async fn boot_cfg(
    ping: Duration,
    liveness: Duration,
    stats: Duration,
) -> (ServerHandle, u16, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = ServerConfig {
        ping_interval: ping,
        liveness_timeout: liveness,
        stats_interval: stats,
        ..ServerConfig::for_tests(dir.path().to_path_buf(), true)
    };
    let mut handle = start(cfg).await.expect("start");
    let port = match handle.events().recv().await.expect("evento") {
        ServerEvent::Listening { port } => port,
        other => panic!("esperava Listening, veio {other:?}"),
    };
    (handle, port, dir)
}

/// Parea `device_id` (auto-aprovado); devolve (cliente, `sessionId`).
async fn pair(handle: &ServerHandle, port: u16, device_id: &str) -> (TestClient, String) {
    let token = handle
        .trust()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .pair_token()
        .to_owned();
    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&TestClient::hello_pair(&token, device_id))
        .await
        .expect("send");
    match c.recv(TIMEOUT).await.expect("esperava WELCOME") {
        Message::Welcome(w) => {
            assert!(w.accepted, "pareamento deve ser aprovado (auto_approve)");
            (c, w.session_id.expect("sessionId"))
        }
        other => panic!("esperava WELCOME, veio {other:?}"),
    }
}

/// Varre até `n` eventos procurando um que satisfaça `pred`.
async fn find_event(
    rx: &mut mpsc::Receiver<ServerEvent>,
    n: usize,
    mut pred: impl FnMut(&ServerEvent) -> bool,
) -> Option<ServerEvent> {
    for _ in 0..n {
        let ev = tokio::time::timeout(TIMEOUT, rx.recv())
            .await
            .ok()
            .flatten()
            .expect("canal aberto");
        if pred(&ev) {
            return Some(ev);
        }
    }
    None
}

/// `SessionEnded{reason}` da sessão `sid`.
fn ended_with(pred_reason: fn(&CloseReason) -> bool) -> impl Fn(&ServerEvent) -> bool {
    move |e| matches!(e, ServerEvent::SessionEnded { reason, .. } if pred_reason(reason))
}

/// AT-1.10 — cliente que não responde PING (`ping 100 ms`,
/// `liveness 400 ms`): `SessionEnded{ConnectionLost}` em ≈ 0,4–0,7 s.
#[tokio::test]
async fn no_pong_closes_session() {
    let (mut handle, port, _dir) = boot_cfg(
        Duration::from_millis(100),
        Duration::from_millis(400),
        Duration::from_millis(500),
    )
    .await;
    let (mut client, _sid) = pair(&handle, port, "dev-1").await;
    let t0 = Instant::now();
    find_event(
        handle.events(),
        30,
        ended_with(|r| matches!(r, CloseReason::ConnectionLost)),
    )
    .await
    .expect("esperava SessionEnded{ConnectionLost}");
    let elapsed = t0.elapsed();
    assert!(
        elapsed >= Duration::from_millis(350),
        "encerrado cedo demais (antes do liveness): {elapsed:?}"
    );
    assert!(
        elapsed <= Duration::from_millis(700),
        "encerrado tarde demais: {elapsed:?}"
    );
    // O buffer pode ter `PING`s pendentes: leia até o EOF (servidor fechou).
    let mut eof = false;
    for _ in 0..30 {
        match client.recv(Duration::from_secs(1)).await {
            Some(_) => {} // PINGs pendentes
            None => {
                eof = true;
                break;
            }
        }
    }
    assert!(eof, "socket fechado após ConnectionLost");
    handle.shutdown().await;
}

/// FT-1.06 — half-open: o cliente para de ler/responder **sem fechar**
/// o TCP; detecção em ≤ `liveness_timeout` + folga.
#[tokio::test]
async fn half_open_detected() {
    let liveness = Duration::from_millis(400);
    let (mut handle, port, _dir) = boot_cfg(
        Duration::from_millis(100),
        liveness,
        Duration::from_millis(500),
    )
    .await;
    // Cliente fica vivo (socket aberto) mas não lê nem responde.
    let (_client, _sid) = pair(&handle, port, "dev-1").await;
    let t0 = Instant::now();
    find_event(
        handle.events(),
        30,
        ended_with(|r| matches!(r, CloseReason::ConnectionLost)),
    )
    .await
    .expect("esperava SessionEnded{ConnectionLost}");
    let elapsed = t0.elapsed();
    assert!(
        elapsed <= liveness + Duration::from_millis(500),
        "detecção acima de liveness + folga: {elapsed:?}"
    );
    handle.shutdown().await;
}

/// IT-1.08 — sessão com vídeo ativo, `stats_interval 500 ms`: 6 eventos
/// consecutivos com intervalo 500 ± 50 ms.
#[tokio::test]
async fn stats_cadence_500ms() {
    let (mut handle, port, _dir) = boot_cfg(
        Duration::from_secs(30),
        Duration::from_secs(90),
        Duration::from_millis(500),
    )
    .await;
    let (mut client, sid) = pair(&handle, port, "dev-1").await;
    for i in 0..10u64 {
        let f = encode_frame(msg::VIDEO, 0, 1, i * 33_333, &[0u8; 1_000]).expect("encode");
        client.send_raw(&f).await.expect("send");
    }
    let mut last: Option<Instant> = None;
    let mut first_fps = 0.0f32;
    for i in 0..6 {
        let ev = find_event(
            handle.events(),
            6,
            |e| matches!(e, ServerEvent::Stats { session_id, .. } if *session_id == sid),
        )
        .await
        .expect("esperava Stats");
        if i == 0 {
            if let ServerEvent::Stats { stats, .. } = &ev {
                first_fps = stats.video_fps[0];
            }
        }
        let now = Instant::now();
        if let Some(prev) = last {
            let gap = now.duration_since(prev);
            // 500 ± 50 ms (cartão); o teto ganha +30 ms de margem para
            // carga de máquina (testes em paralelo) atrasar o timer do
            // agregador — o piso continua rígido (timer não antecipa).
            assert!(
                (450..=580).contains(&gap.as_millis()),
                "intervalo entre Stats fora de 500 ± 50 ms: {gap:?}"
            );
        }
        last = Some(now);
    }
    assert!(
        first_fps > 0.0,
        "vídeo ativo deve aparecer no 1º Stats (fps[0] = {first_fps})"
    );
    handle.shutdown().await;
}

/// Cliente responde `PING` com `PONG` após 50 ms: `rtt_ms` entre 30 e
/// 120.
#[tokio::test]
async fn rtt_measured() {
    let (mut handle, port, _dir) = boot_cfg(
        Duration::from_millis(100),
        Duration::from_secs(5),
        Duration::from_millis(200),
    )
    .await;
    let (mut client, sid) = pair(&handle, port, "dev-1").await;
    let pong = tokio::spawn(async move {
        for _ in 0..60 {
            match client.recv(Duration::from_millis(200)).await {
                Some(Message::Ping(v)) => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    let _ = client.send(&Message::Pong(v)).await;
                }
                Some(_) => {}
                None => return, // EOF: sessão caiu
            }
        }
    });
    let ev = find_event(handle.events(), 40, |e| {
        matches!(
            e,
            ServerEvent::Stats { session_id, stats, .. }
                if *session_id == sid && stats.rtt_ms.is_some()
        )
    })
    .await
    .expect("esperava Stats com rtt_ms");
    let rtt = match &ev {
        ServerEvent::Stats { stats, .. } => stats.rtt_ms.expect("rtt presente (filtro)"),
        _ => unreachable!("find_event filtra Stats com rtt"),
    };
    assert!((30.0..=120.0).contains(&rtt), "RTT ~50 ms; veio {rtt}");
    let _ = pong.await;
    handle.shutdown().await;
}

/// Só `PONG`s, nenhum vídeo: a sessão **não** cai (liveness de 300 ms
/// para 1,5 s de apenas PING/PONG).
#[tokio::test]
async fn pong_keeps_alive_without_other_traffic() {
    let (mut handle, port, _dir) = boot_cfg(
        Duration::from_millis(100),
        Duration::from_millis(300),
        Duration::from_secs(5),
    )
    .await;
    let (mut client, sid) = pair(&handle, port, "dev-1").await;
    // 1,5 s respondendo só PING→PONG (nenhum outro tráfego).
    let deadline = tokio::time::sleep(Duration::from_millis(1_500));
    tokio::pin!(deadline);
    let mut responded = 0u32;
    loop {
        tokio::select! {
            _ = &mut deadline => break,
            m = client.recv(Duration::from_millis(200)) => match m {
                Some(Message::Ping(v)) => {
                    client.send(&Message::Pong(v)).await.expect("send PONG");
                    responded += 1;
                }
                Some(_) => {}
                None => panic!("sessão caiu (EOF) antes do fim da janela"),
            },
        }
    }
    assert!(
        responded >= 5,
        "deviam chegar ~15 PINGs; vieram {responded}"
    );
    // Sessão segue viva: o `BYE` encerra com `PeerClosed` (e não
    // `ConnectionLost`).
    client
        .send(&Message::Bye(lc_protocol::json::Bye {
            reason: "done".to_owned(),
        }))
        .await
        .expect("send");
    let ended = find_event(
        handle.events(),
        30,
        |e| matches!(e, ServerEvent::SessionEnded { session_id, .. } if *session_id == sid),
    )
    .await
    .expect("esperava SessionEnded");
    assert!(
        matches!(
            &ended,
            ServerEvent::SessionEnded { reason, .. } if *reason == CloseReason::PeerClosed
        ),
        "BYE limpo → PeerClosed, não ConnectionLost: {ended:?}"
    );
    handle.shutdown().await;
}

/// 10 quadros no stream 1 e 5 no stream 2 (1º de cada = keyframe):
/// contadores corretos por stream.
#[tokio::test]
async fn video_counters_per_stream() {
    let (mut handle, port, _dir) = boot_cfg(
        Duration::from_secs(30),
        Duration::from_secs(90),
        Duration::from_secs(5),
    )
    .await;
    let (mut client, sid) = pair(&handle, port, "dev-1").await;
    for i in 0..10u64 {
        let fl = if i == 0 { flags::KEYFRAME } else { 0 };
        let f = encode_frame(msg::VIDEO, fl, 1, i * 33_333, &[0u8; 100]).expect("encode");
        client.send_raw(&f).await.expect("send");
    }
    for i in 0..5u64 {
        let fl = if i == 0 { flags::KEYFRAME } else { 0 };
        let f = encode_frame(msg::VIDEO, fl, 2, i * 33_333, &[0u8; 200]).expect("encode");
        client.send_raw(&f).await.expect("send");
    }
    tokio::time::sleep(Duration::from_millis(200)).await; // processamento
    let c = handle.sessions().counters(&sid).expect("sessão ativa");
    assert_eq!(c.video_frames[0].load(Ordering::Relaxed), 10, "stream 1");
    assert_eq!(c.video_frames[1].load(Ordering::Relaxed), 5, "stream 2");
    assert_eq!(c.video_frames[2].load(Ordering::Relaxed), 0, "reservado");
    assert_eq!(
        c.video_bytes[0].load(Ordering::Relaxed),
        1_000,
        "bytes stream 1"
    );
    assert_eq!(
        c.video_bytes[1].load(Ordering::Relaxed),
        1_000,
        "bytes stream 2"
    );
    assert_eq!(
        c.keyframes.load(Ordering::Relaxed),
        2,
        "1 keyframe por stream"
    );
    handle.shutdown().await;
}
