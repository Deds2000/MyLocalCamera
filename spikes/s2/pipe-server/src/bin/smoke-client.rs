// Smoke client do spike S2 — cliente de pipe usado para validar o servidor
// ANTES do teste real com a DLL (Frame Server). Roda no MESMO usuário do
// servidor (validação de protocolo/vazão; o teste de acesso cross-conta é o
// do humano com a DLL).
//
// Uso: smoke-client [--duration 10]
//
// Conecta em \\.\pipe\LocalCameraSpike, lê mensagens (header + NV12), valida
// o cabeçalho e o seq (contagem de perdas por gap) e responde com ack
// {seq, read_pts_us} para a medição de latência do servidor.

use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    CloseHandle, GetLastError, GENERIC_READ, GENERIC_WRITE,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_MODE, OPEN_EXISTING, ReadFile, WriteFile,
};

use s2_pipe_server::protocol::{
    read_header, write_ack, qpc_us, ACK_SIZE, FORMAT_NV12, HEADER_SIZE, MSG_SIZE,
};
use s2_pipe_server::pipe::PIPE_NAME;

fn main() {
    let mut duration = 10u64;
    let mut it = std::env::args().skip(1);
    while let Some(s) = it.next() {
        if s == "--duration" {
            duration = it.next().and_then(|v| v.parse().ok()).unwrap_or(10);
        }
    }

    let deadline = Instant::now() + Duration::from_secs(duration);
    let mut buf = vec![0u8; MSG_SIZE];
    let mut ack = [0u8; ACK_SIZE];

    let mut hpipe: Option<windows::Win32::Foundation::HANDLE> = None;

    let mut frames = 0u64;
    let mut dropped = 0u64;
    let mut last_seq: Option<u64> = None;
    let mut bytes = 0u64;
    let mut latencies: Vec<u64> = Vec::new();
    let mut start: Option<Instant> = None;

    loop {
        if Instant::now() >= deadline {
            break;
        }
        if hpipe.is_none() {
            let h = unsafe {
                CreateFileW(
                    windows::core::w!(r"\\.\pipe\LocalCameraSpike"),
                    GENERIC_READ.0 | GENERIC_WRITE.0,
                    FILE_SHARE_MODE(0),
                    None,
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    None,
                )
            };
            match h {
                Ok(h) => {
                    hpipe = Some(h);
                    println!("[+] conectado em {PIPE_NAME}");
                }
                Err(_) => {
                    let e = unsafe { GetLastError() };
                    println!("[.] pipe indisponível (win32={}); tentando de novo em 500ms", e.0);
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        }

        let h = hpipe.unwrap();
        match unsafe { ReadFile(h, Some(&mut buf), None, None) } {
            Ok(()) => {
                let hdr = read_header(&buf[..HEADER_SIZE]);
                if hdr.w != 1920 || hdr.h != 1080 || hdr.format != FORMAT_NV12 {
                    eprintln!("[!] header inválido: {hdr:?}");
                }
                if let Some(last) = last_seq {
                    if hdr.seq > last + 1 {
                        dropped += hdr.seq - last - 1;
                    }
                }
                last_seq = Some(hdr.seq);
                frames += 1;
                bytes += MSG_SIZE as u64;

                let read_pts = qpc_us();
                if read_pts >= hdr.pts_us {
                    latencies.push(read_pts - hdr.pts_us);
                }
                write_ack(&mut ack, hdr.seq, read_pts);
                if let Err(e) = unsafe { WriteFile(h, Some(&ack), None, None) } {
                    println!("[!] ack falhou: {e}");
                    let _ = unsafe { CloseHandle(h) };
                    hpipe = None;
                }
                if start.is_none() {
                    start = Some(Instant::now());
                }
            }
            Err(e) => {
                println!("[~] leitura terminou ({e}); reconectando ...");
                let _ = unsafe { CloseHandle(h) };
                hpipe = None;
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }

    if let Some(h) = hpipe {
        let _ = unsafe { CloseHandle(h) };
    }

    let secs = start.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0);
    let mbps = if secs > 0.0 { bytes as f64 / 1e6 / secs } else { 0.0 };
    let mut v = latencies.clone();
    v.sort_unstable();
    let p50 = v.get(v.len() / 2).copied().unwrap_or(0);
    let p95 = v.get((v.len() * 95 / 100).min(v.len().saturating_sub(1))).copied().unwrap_or(0);
    println!(
        "\n=== SMOKE CLIENT (mesmo usuário) ===\nquadros={} perdidos(gap)={} {:.1} MB/s lat p50/p95={}us/{}us",
        frames,
        dropped,
        mbps,
        p50,
        p95
    );
}
