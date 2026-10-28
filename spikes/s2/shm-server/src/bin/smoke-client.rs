// Smoke client do shm-server — valida o MECANISMO (mapping + evento +
// protocolo) no mesmo usuário/sessão. O teste real (leitura pela DLL do Frame
// Server em Global\) é feito pelo humano, ver docs/spikes/S2.md.
//
// Uso: smoke-client [--duration 10] [--ns local|global]
//
// Abre `Local\LocalCameraSpike` (padrão local) + `...Evt`, aguarda o evento,
// lê o header, conta perdas por gap de seq e mede latência escrita->leitura.

use std::time::{Duration, Instant};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GetLastError, WAIT_OBJECT_0};
use windows::Win32::System::Memory::{
    MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_COPY,
};
use windows::Win32::System::Threading::{
    OpenEventW, WaitForSingleObject, SYNCHRONIZATION_ACCESS_RIGHTS,
};

use s2_pipe_server::protocol::{qpc_us, FORMAT_NV12, HEADER_SIZE, MSG_SIZE};

fn wide(s: &str) -> (Vec<u16>, PCWSTR) {
    let v: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    let p = PCWSTR(v.as_ptr());
    (v, p)
}

fn main() {
    let mut duration = 10u64;
    let mut ns = "local";
    let mut it = std::env::args().skip(1);
    while let Some(s) = it.next() {
        if s == "--duration" {
            duration = it.next().and_then(|v| v.parse().ok()).unwrap_or(10);
        } else if s == "--ns" {
            ns = match it.next().as_deref() {
                Some("global") => "global",
                _ => "local",
            };
        }
    }

    // prefixo sensível a caixa: exatamente Global\ ou Local\
    let prefix = if ns == "global" { "Global" } else { "Local" };
    let map_name = format!("{}\\LocalCameraSpike", prefix);
    let evt_name = format!("{}\\LocalCameraSpikeEvt", prefix);
    let (_map_w, map_pc) = wide(&map_name);
    let (_evt_w, evt_pc) = wide(&evt_name);

    let hmap = unsafe { OpenFileMappingW(FILE_MAP_COPY.0, false, map_pc) };
    let hevt = unsafe {
        OpenEventW(SYNCHRONIZATION_ACCESS_RIGHTS(0x00100000), false, evt_pc)
    };

    let hmap = match hmap {
        Ok(h) => h,
        Err(_) => {
            let e = unsafe { GetLastError() };
            println!(
                "[!] OpenFileMappingW({map_name}) indisponível (win32={}); tentando de novo em 500ms",
                e.0
            );
            let deadline = Instant::now() + Duration::from_secs(duration);
            // tenta de novo até o prazo (o servidor pode estar subindo)
            loop {
                let h = unsafe { OpenFileMappingW(FILE_MAP_COPY.0, false, map_pc) };
                if let Ok(h) = h {
                    let hevt = unsafe {
                        OpenEventW(SYNCHRONIZATION_ACCESS_RIGHTS(0x00100000), false, evt_pc)
                    };
                    let hevt = match hevt {
                        Ok(h2) => h2,
                        Err(_) => windows::Win32::Foundation::HANDLE(std::ptr::null_mut()),
                    };
                    run(h, hevt, ns, duration);
                    return;
                }
                if Instant::now() >= deadline {
                    println!("[=] desistindo; objeto nunca apareceu");
                    return;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    };
    let hevt = match hevt {
        Ok(h) => h,
        Err(_) => {
            let e = unsafe { GetLastError() };
            println!("[!] OpenEventW({evt_name}) falhou (win32={})", e.0);
            return;
        }
    };

    run(hmap, hevt, ns, duration);
}

fn run(hmap: windows::Win32::Foundation::HANDLE, hevt: windows::Win32::Foundation::HANDLE, ns: &str, duration: u64) {
    let view = unsafe { MapViewOfFile(hmap, FILE_MAP_COPY, 0, 0, MSG_SIZE as usize) };
    if view.Value.is_null() {
        let e = unsafe { GetLastError() };
        println!("[!] MapViewOfFile falhou (win32={})", e.0);
        return;
    }
    let base = view.Value.cast::<u8>();

    let deadline = Instant::now() + Duration::from_secs(duration);
    let mut frames = 0u64;
    let mut dropped = 0u64;
    let mut last_seq: Option<u64> = None;
    let mut bytes = 0u64;
    let mut latencies: Vec<u64> = Vec::new();
    let mut start: Option<Instant> = None;

    println!("[+] lendo {} ...", format!("{}\\LocalCameraSpike", ns));

    while Instant::now() < deadline {
        let r = unsafe { WaitForSingleObject(hevt, 100) };
        if r != WAIT_OBJECT_0 {
            continue;
        }
        let hdr = unsafe { read_header_from(base) };
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
        if start.is_none() {
            start = Some(Instant::now());
        }
    }

    unsafe {
        let _ = UnmapViewOfFile(view);
        let _ = CloseHandle(hevt);
        let _ = CloseHandle(hmap);
    }

    let secs = start.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0);
    let mbps = if secs > 0.0 { bytes as f64 / 1e6 / secs } else { 0.0 };
    let mut v = latencies.clone();
    v.sort_unstable();
    let p50 = v.get(v.len() / 2).copied().unwrap_or(0);
    let p95 = v.get((v.len() * 95 / 100).min(v.len().saturating_sub(1))).copied().unwrap_or(0);
    println!(
        "\n=== SMOKE CLIENT SHM ({ns}) ===\nquadros={} perdidos(gap)={} {:.1} MB/s lat p50/p95={}us/{}us",
        frames, dropped, mbps, p50, p95
    );
}

unsafe fn read_header_from(base: *const u8) -> s2_pipe_server::protocol::FrameHeader {
    // lê os 28 bytes do header numa stack buffer
    let mut buf = [0u8; HEADER_SIZE];
    std::ptr::copy_nonoverlapping(base, buf.as_mut_ptr(), HEADER_SIZE);
    s2_pipe_server::protocol::read_header(&buf)
}
