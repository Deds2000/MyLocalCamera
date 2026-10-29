//! PT-1.01 — vazão do parser `FrameReader`: 1 GiB de quadros `VIDEO` de
//! 64 KiB em 1 thread; meta ≥ 500 MiB/s.
//!
//! Rode: `cargo test --release -p lc-protocol --test perf -- --ignored --nocapture`

use std::time::Instant;

use lc_protocol::{encode_frame, msg, FrameReader};

/// Payload de cada quadro (64 KiB).
const PAYLOAD: usize = 64 * 1024;
/// Volume total a processar (1 GiB).
const TOTAL: u64 = 1024 * 1024 * 1024;

#[test]
#[ignore = "perf: rode com --release --ignored --nocapture"]
fn frame_reader_throughput_ge_500_mib_s() {
    let payload = vec![0xAB; PAYLOAD];
    let frame = encode_frame(msg::VIDEO, 0, 1, 0, &payload).expect("encode");
    let frame_len = frame.len() as u64;
    let total_frames = TOTAL.div_ceil(frame_len);

    let mut reader = FrameReader::new();
    let mut frames = 0u64;
    let start = Instant::now();
    for _ in 0..total_frames {
        // Nesse padrão (1 quadro completo por push), o leitor devolve
        // exatamente 1 quadro por chamada.
        frames += reader.push(&frame).expect("leitura").len() as u64;
    }
    let secs = start.elapsed().as_secs_f64();
    let bytes = total_frames * frame_len;
    let mibps = bytes as f64 / (1024.0 * 1024.0) / secs;
    println!("FrameReader: {frames} quadros de 64 KiB em {secs:.2}s = {mibps:.0} MiB/s");
    assert!(mibps >= 500.0, "vazão {mibps:.0} MiB/s < meta 500 MiB/s");
}
