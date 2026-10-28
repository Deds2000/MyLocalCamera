// Servidor de memória compartilhada do spike S2 (cartão
// docs/tasks/fase-0/F0-S2.md, passo 3).
//
// Cria `Global\LocalCameraSpike` (CreateFileMapping) + evento de sinalização
// `Global\LocalCameraSpikeEvt` (CreateEvent, auto-reset) e publica quadros
// NV12 1080p a 30 fps: escreve {header 28 B, payload} na view mapeada e chama
// SetEvent. O leitor (DLL/spike) faz WaitForSingleObject e lê a view.
//
// USO:
//   s2-shm-server [--duration 60] [--rate 30] [--ns global|local] [--out s2-shm.json]
//
// --ns global (padrão): objetos em `Global\` — visíveis entre sessões, mas a
//   CRIAÇÃO exige SeCreateGlobalPrivilege, que um usuário comum NÃO tem
//   (verificado nesta máquina: ERROR_ACCESS_DENIED, bin priv-test do
//   pipe-server). O erro é relatado de forma clara e o JSON registra
//   "created": false — esse é exatamente o resultado esperado do teste do
//   cartão ("Criar o objeto sem elevação").
// --ns local: objetos em `Local\` (namespace da sessão) — usado pelo
//   smoke-client para validar o MECANISMO do protocolo (mapping+evento)
//   quando a criação Global\ é negada.
//
// Saída: exit 0 = rodou; exit 2 = objeto não pôde ser criado (achado do spike).

use std::time::{Duration, Instant};

use windows::core::PCWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Memory::{
    CreateFileMappingW, FILE_MAP_WRITE, MapViewOfFile, PAGE_READWRITE, UnmapViewOfFile,
};
use windows::Win32::System::Threading::{CreateEventW, SetEvent};

use s2_pipe_server::frame;
use s2_pipe_server::protocol::{
    qpc_us, write_header, FPS, FRAME_H, FRAME_W, FORMAT_NV12, HEADER_SIZE, MSG_SIZE, TICK_US,
};

struct Args {
    duration_s: u64,
    rate: u64, // 0 = back-to-back
    ns: &'static str,
    out: String,
}

fn parse_args() -> Args {
    let mut a = Args {
        duration_s: 60,
        rate: FPS,
        ns: "global",
        out: "s2-shm.json".into(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(s) = it.next() {
        match s.as_str() {
            "--duration" => a.duration_s = it.next().and_then(|v| v.parse().ok()).unwrap_or(60),
            "--rate" => a.rate = it.next().and_then(|v| v.parse().ok()).unwrap_or(FPS),
            "--ns" => a.ns = match it.next().as_deref() {
                Some("local") => "local",
                _ => "global",
            },
            "--out" => a.out = it.next().unwrap_or_else(|| "s2-shm.json".into()),
            _ => eprintln!("argumento ignorado: {s}"),
        }
    }
    a
}

fn wide(s: &str) -> (Vec<u16>, PCWSTR) {
    let v: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    let p = PCWSTR(v.as_ptr());
    (v, p)
}

fn failed_json(args: &Args, map_err: Option<String>, evt_err: Option<String>) -> String {
    let map_e = map_err
        .map(|e| format!("\"{e}\""))
        .unwrap_or_else(|| "null".into());
    let evt_e = evt_err
        .map(|e| format!("\"{e}\""))
        .unwrap_or_else(|| "null".into());
    format!(
        "{{\n  \"channel\": \"shm\",\n  \"namespace\": \"{}\",\n  \"created\": false,\n  \"map_error\": {},\n  \"event_error\": {},\n  \"note\": \"usuário comum não tem SeCreateGlobalPrivilege (esperado)\"\n}}",
        args.ns, map_e, evt_e
    )
}

fn ok_json(args: &Args, frames: u64, late: u64, bytes: u64, duration_s: f64) -> String {
    let mbps = if duration_s > 0.0 { bytes as f64 / 1e6 / duration_s } else { 0.0 };
    format!(
        "{{\n  \"channel\": \"shm\",\n  \"namespace\": \"{}\",\n  \"created\": true,\n  \"duration_s\": {},\n  \"frames_written\": {},\n  \"bytes_written\": {},\n  \"throughput_mbps\": {:.1},\n  \"late_ticks\": {}\n}}",
        args.ns, duration_s, frames, bytes, mbps, late
    )
}

fn run(args: &Args) -> i32 {
    // O prefixo do namespace é SENSÍVEL A CAIXA: tem que ser exatamente
    // `Global\` ou `Local\` (do contrário o nome vira um objeto literal com
    // barra no namespace local e falha com ERROR_PATH_NOT_FOUND).
    let prefix = if args.ns == "global" { "Global" } else { "Local" };
    let map_name = format!("{}\\LocalCameraSpike", prefix);
    let evt_name = format!("{}\\LocalCameraSpikeEvt", prefix);
    let (_map_w, map_pc) = wide(&map_name);
    let (_evt_w, evt_pc) = wide(&evt_name);

    println!("shm: {map_name} + evento {evt_name} ({}) por {} s", args.ns, args.duration_s);

    // 1) mapping (é a criação que exige SeCreateGlobalPrivilege em Global\).
    let mut map_err = None;
    let hmap = unsafe {
        match CreateFileMappingW(
            windows::Win32::System::Threading::GetCurrentProcess(),
            None,
            PAGE_READWRITE,
            0,
            MSG_SIZE as u32,
            map_pc,
        ) {
            Ok(h) => h,
            Err(e) => {
                map_err = Some(e.to_string());
                windows::Win32::Foundation::HANDLE(std::ptr::null_mut())
            }
        }
    };

    // 2) evento (criar evento em Global\ NÃO exige o privilégio, por
    //    documentação — mas reportamos o que de fato acontece).
    let mut evt_err = None;
    let hevt = unsafe {
        match CreateEventW(None, false, false, evt_pc) {
            Ok(h) => h,
            Err(e) => {
                evt_err = Some(e.to_string());
                windows::Win32::Foundation::HANDLE(std::ptr::null_mut())
            }
        }
    };

    if hmap.is_invalid() || hevt.is_invalid() {
        let json = failed_json(args, map_err.clone(), evt_err.clone());
        if let Some(e) = &map_err {
            println!("[!] CreateFileMappingW falhou: {e}");
        }
        if let Some(e) = &evt_err {
            println!("[!] CreateEventW falhou: {e}");
        }
        if hmap.is_invalid() {
            println!("[=] RESULTADO ESPERADO para usuário comum: criação em Global\\ é negada");
        }
        if let Err(e) = std::fs::write(&args.out, &json) {
            eprintln!("[!] não consegui gravar {}: {e}", args.out);
        }
        println!("\n=== RESUMO (shm) ===\n{json}");
        return 2;
    }

    // 3) view mapeada para escrita.
    let view = unsafe { MapViewOfFile(hmap, FILE_MAP_WRITE, 0, 0, 0) };
    if view.Value.is_null() {
        let e = unsafe { windows::Win32::Foundation::GetLastError() };
        println!("[!] MapViewOfFile falhou (win32={})", e.0);
        return 1;
    }

    println!("[+] objeto criado; publicando quadros NV12 1080p ...");
    let start = Instant::now();
    let mut seq: u64 = 0;
    let mut msg = vec![0u8; MSG_SIZE];
    let mut frames = 0u64;
    let mut late = 0u64;

    while start.elapsed() < Duration::from_secs(args.duration_s) {
        seq += 1;
        let pts = qpc_us();
        frame::fill_nv12(&mut msg[HEADER_SIZE..], seq);
        write_header(&mut msg[..HEADER_SIZE], FRAME_W, FRAME_H, FORMAT_NV12, seq, pts);

        unsafe {
            std::ptr::copy_nonoverlapping(msg.as_ptr(), view.Value.cast::<u8>(), MSG_SIZE);
            let _ = SetEvent(hevt);
        }
        frames += 1;

        if args.rate > 0 {
            let ideal = start + Duration::from_micros(frames * TICK_US);
            let now = Instant::now();
            if now < ideal {
                std::thread::sleep(ideal - now);
            } else {
                late += 1;
            }
        }
    }

    unsafe {
        let _ = UnmapViewOfFile(view);
        let _ = CloseHandle(hevt);
        let _ = CloseHandle(hmap);
    }

    let json = ok_json(args, frames, late, frames as u64 * MSG_SIZE as u64, args.duration_s as f64);
    if let Err(e) = std::fs::write(&args.out, &json) {
        eprintln!("[!] não consegui gravar {}: {e}", args.out);
    }
    println!("\n=== RESUMO (shm) ===\n{json}");
    0
}

fn main() {
    let args = parse_args();
    std::process::exit(run(&args));
}
