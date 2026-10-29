//! Testes de integração da Fase 1 (F1-T13): fluxo completo de vídeo +
//! áudio (IT-1.01), persistência de confiança após reinício (AT-1.15),
//! apagar `trust.json` com o servidor ligado (FT-1.09) e latência p95
//! de handshake (PT-1.02).

#[allow(dead_code)]
mod common;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use common::TestClient;
use lc_protocol::json::{AudioHello, Reason, StreamConfig, Welcome};
use lc_protocol::{encode_frame, msg, Message};
use lc_server::config::ServerConfig;
use lc_server::events::ServerEvent;
use lc_server::{start, ServerHandle};

/// Prazo dos `recv` dos clientes de teste.
const TIMEOUT: Duration = Duration::from_secs(3);

/// `hd30.h264` (300 AUs, 10 s de 1080p @ 30 fps), relativo ao crate.
fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/hd30.h264")
}

/// Sobe o servidor em `dir` e devolve (handle, porta efetiva).
async fn start_on(dir: &std::path::Path, auto_approve: bool) -> (ServerHandle, u16) {
    let cfg = ServerConfig::for_tests(dir.to_path_buf(), auto_approve);
    let mut handle = start(cfg).await.expect("start");
    let port = match handle.events().recv().await.expect("Listening") {
        ServerEvent::Listening { port } => port,
        other => panic!("esperava Listening, veio {other:?}"),
    };
    (handle, port)
}

/// Servidor novo em um tempdir (dir mantido vivo até o fim do teste).
async fn boot(auto_approve: bool) -> (ServerHandle, u16, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let (handle, port) = start_on(dir.path(), auto_approve).await;
    (handle, port, dir)
}

/// Token de pareamento da loja em execução.
fn pair_token(handle: &ServerHandle) -> String {
    handle
        .trust()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .pair_token()
        .to_owned()
}

/// Recebe a próxima mensagem e asser que é um `WELCOME`.
async fn take_welcome(c: &mut TestClient) -> Welcome {
    match c.recv(TIMEOUT).await.expect("esperava WELCOME") {
        Message::Welcome(w) => w,
        other => panic!("esperava WELCOME, veio {other:?}"),
    }
}

/// Conecta, envia `msg` e asser que a resposta é um `WELCOME`.
async fn hello(port: u16, msg: Message) -> (TestClient, Welcome) {
    let mut c = TestClient::connect(port).await.expect("connect");
    c.send(&msg).await.expect("send");
    let w = take_welcome(&mut c).await;
    (c, w)
}

// IT-1.01 — fluxo completo de 10 s: 300 AUs de `hd30.h264` @ 30 fps no
// socket principal + 1000 blocos de 960 B @ 100/s no socket de áudio;
// contadores: ≈ 300 quadros (±5) e ≈ 1000 blocos (±20).
#[tokio::test]
async fn full_flow_10s() {
    let (mut handle, port, _dir) = boot(true).await;
    let data = std::fs::read(fixture_path()).expect("fixture hd30.h264");
    let aus = lc_testkit::annexb::split_access_units(&data);
    assert_eq!(aus.len(), 300, "fixture deve ter 300 AUs");
    let flags: Vec<u8> = aus
        .iter()
        .map(|au| {
            let n = lc_testkit::annexb::nal_types(&data[au.clone()]);
            let idr = n.contains(&5) as u8;
            let params = (n.contains(&7) && n.contains(&8)) as u8 * 2;
            idr | params
        })
        .collect();

    let (mut main, w) = hello(
        port,
        TestClient::hello_pair(&pair_token(&handle), "dev-it101"),
    )
    .await;
    assert!(w.accepted, "pareamento aprovado (auto_approve)");
    let sid = w.session_id.expect("sessionId");
    let ticket = w.audio_ticket.expect("audioTicket");

    main.send(&Message::StreamConfig(StreamConfig {
        stream: 1,
        width: 1920,
        height: 1080,
        fps: 30,
        rotation_hint: 0,
        facing: "back".to_owned(),
        profile: None,
    }))
    .await
    .expect("send");

    let (audio, aw) = hello(
        port,
        Message::AudioHello(AudioHello {
            session_id: sid.clone(),
            ticket,
        }),
    )
    .await;
    assert!(aw.accepted, "socket de áudio aceito");

    let counters = handle.sessions().counters(&sid).expect("sessão ativa");

    // O vídeo termina (~9,9 s) antes do áudio (~10 s); fechar a principal
    // no fim do vídeo encerraria a sessão e cortaria o áudio em
    // transmissão — a task de vídeo segura o socket aberto até o áudio
    // terminar.
    let (audio_done_tx, audio_done_rx) = tokio::sync::oneshot::channel::<()>();

    let video = tokio::spawn(async move {
        let mut c = main;
        let mut tick = tokio::time::interval(Duration::from_millis(1000 / 30));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Burst);
        for (i, au) in aus.iter().enumerate() {
            tick.tick().await;
            let pts = i as u64 * 1_000_000 / 30;
            let frame =
                encode_frame(msg::VIDEO, flags[i], 1, pts, &data[au.clone()]).expect("encode");
            c.send_raw(&frame).await.expect("send");
        }
        let _ = audio_done_rx.await; // mantém a sessão viva até o áudio fechar
    });

    let block = vec![0u8; 960];
    let audio_task = tokio::spawn(async move {
        let mut c = audio;
        let mut tick = tokio::time::interval(Duration::from_millis(10));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Burst);
        for i in 0..1000u64 {
            tick.tick().await;
            let frame = encode_frame(msg::AUDIO, 0, 3, i * 10_000, &block).expect("encode");
            c.send_raw(&frame).await.expect("send");
        }
        drop(c); // fecha o socket de áudio
        let _ = audio_done_tx.send(());
    });

    audio_task.await.expect("audio");
    video.await.expect("video");

    // Polling curto (50 ms, até 2 s) para os contadores assentarem.
    let deadline = Instant::now() + Duration::from_secs(2);
    let (vf, ab) = loop {
        let vf = counters.video_frames[0].load(Ordering::Relaxed);
        let ab = counters.audio_blocks.load(Ordering::Relaxed);
        while handle.events().try_recv().is_ok() {}
        let in_range = (295..=305).contains(&vf) && (980..=1020).contains(&ab);
        if in_range || Instant::now() >= deadline {
            break (vf, ab);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    assert!(
        (295..=305).contains(&vf),
        "video_frames[0] = {vf} (esperado ≈ 300)"
    );
    assert!(
        (980..=1020).contains(&ab),
        "audio_blocks = {ab} (esperado ≈ 1000)"
    );

    handle.shutdown().await;
}

// AT-1.15 — restart com o mesmo `data_dir`: dispositivo confiável é
// reconhecido pela `deviceKey` e **não** recebe chave nova.
#[tokio::test]
async fn restart_server_keeps_trusted_devices() {
    let (mut handle, port, dir) = boot(true).await;

    let (_, w) = hello(
        port,
        TestClient::hello_pair(&pair_token(&handle), "dev-at115"),
    )
    .await;
    assert!(w.accepted, "pareamento aprovado (auto_approve)");
    let key = w.device_key.expect("deviceKey (ApproveAlways)");

    handle.shutdown().await;

    let (mut handle2, port2) = start_on(dir.path(), true).await; // mesmo data_dir
    let (_, w2) = hello(port2, TestClient::hello_device("dev-at115", &key)).await;
    assert!(w2.accepted, "dispositivo sobrevive ao restart");
    assert_eq!(w2.device_key, None, "reconexão não emite nova deviceKey");

    handle2.shutdown().await;
}

// FT-1.09 — apagar `trust.json` com o servidor ligado.
// Comportamento documentado: a loja é **memória + disco** — apagar o
// arquivo não afeta o servidor em execução (o próximo `hello_device`
// segue aceito, sem panic; e, por atualizar `last_seen`, **regrava**
// o arquivo). Ao reiniciar com o mesmo `data_dir` e o arquivo apagado,
// a loja nasce nova (token/pcId novos, zero dispositivos): o antigo
// device é `revoked` e o antigo token é `bad_token`.
#[tokio::test]
async fn trust_file_deleted_while_running() {
    let (mut handle, port, dir) = boot(true).await;
    let token = pair_token(&handle);
    let (_, w) = hello(port, TestClient::hello_pair(&token, "dev-ft109")).await;
    assert!(w.accepted);
    let key = w.device_key.expect("deviceKey");

    std::fs::remove_file(dir.path().join("trust.json")).expect("apagar trust.json");

    // (a) Servidor AINDA LIGADO: a loja em memória aceita (sem panic).
    let (_, w2) = hello(port, TestClient::hello_device("dev-ft109", &key)).await;
    assert!(
        w2.accepted,
        "loja em memória segue aceitando com o servidor ligado"
    );

    // Um `hello_device` bem-sucedido atualiza `last_seen` e **regrava**
    // `trust.json`; apaga de novo para o restart ver uma loja vazia.
    std::fs::remove_file(dir.path().join("trust.json")).expect("apagar trust.json de novo");

    handle.shutdown().await;

    // (b) Reinício com o mesmo `data_dir`: loja nova → device revogado…
    let (mut handle2, port2) = start_on(dir.path(), true).await;
    let (_, w3) = hello(port2, TestClient::hello_device("dev-ft109", &key)).await;
    assert!(!w3.accepted, "após restart, device antigo não é aceito");
    assert_eq!(w3.reason, Some(Reason::Revoked));

    // (c) …e o token antigo também não serve mais.
    let (_, w4) = hello(port2, TestClient::hello_pair(&token, "dev-ft109-novo")).await;
    assert!(!w4.accepted);
    assert_eq!(w4.reason, Some(Reason::BadToken));

    handle2.shutdown().await;
}

// PT-1.02 — 500 handshakes de `hello_device` a 50/s (~10 s): nenhuma
// falha e p95 ≤ 20 ms. `#[ignore]` porque consome ~10 s.
#[tokio::test]
#[ignore = "perf: rode com --ignored --nocapture"]
async fn handshake_p95_under_20ms() {
    let (mut handle, port, _dir) = boot(true).await;

    let (_, w) = hello(
        port,
        TestClient::hello_pair(&pair_token(&handle), "dev-pt102"),
    )
    .await;
    assert!(w.accepted);
    let key = w.device_key.expect("deviceKey");

    let worker = tokio::spawn(async move {
        let mut lat: Vec<Duration> = Vec::with_capacity(500);
        let mut errs: Vec<String> = Vec::new();
        let start = tokio::time::Instant::now();
        for i in 0u64..500 {
            tokio::time::sleep_until(start + Duration::from_millis(i * 20)).await;
            let t0 = Instant::now();
            let res: Result<Welcome, String> = async {
                let mut cl = TestClient::connect(port)
                    .await
                    .map_err(|e| format!("i={i}: connect: {e}"))?;
                cl.send(&TestClient::hello_device("dev-pt102", &key))
                    .await
                    .map_err(|e| format!("i={i}: send: {e}"))?;
                match cl.recv(TIMEOUT).await {
                    Some(Message::Welcome(wel)) => Ok(wel),
                    _ => Err(format!("i={i}: sem WELCOME a tempo")),
                }
            }
            .await;
            match res {
                Ok(wel) if wel.accepted => lat.push(t0.elapsed()),
                Ok(wel) => errs.push(format!("i={i}: recusa {:?}", wel.reason)),
                Err(e) => errs.push(e),
            }
        }
        (lat, errs)
    });

    // Drena os eventos CONTINUAMENTE até a worker terminar: o canal tem
    // capacidade 64 e, sem drenagem, o servidor trava em
    // `events_tx.send` e o handshake nunca conclui.
    let (mut lat, errs) = loop {
        if worker.is_finished() {
            break worker.await.expect("worker");
        }
        tokio::select! {
            _ev = handle.events().recv() => {}
            _ = tokio::time::sleep(Duration::from_millis(5)) => {}
        }
    };

    assert!(errs.is_empty(), "falhas: {:?}", &errs[..errs.len().min(5)]);
    assert_eq!(lat.len(), 500, "esperava 500 latências");
    lat.sort();
    let p95 = lat[(500 * 95) / 100];
    println!("p95 = {p95:?} | n = 500");
    assert!(p95 <= Duration::from_millis(20), "p95 {p95:?} > 20 ms");

    handle.shutdown().await;
}
