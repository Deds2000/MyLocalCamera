// Servidor de named pipe do spike S2 (cartão docs/tasks/fase-0/F0-S2.md).
//
// Cria \\.\pipe\LocalCameraSpike com PIPE_REJECT_REMOTE_CLIENTS e uma DACL que
// dá acesso ao usuário atual + NT AUTHORITY\LOCAL SERVICE (conta do Frame
// Server). Envia quadros NV12 1080p sintéticos com cabeçalho {w,h,format,seq,
// ptsUs} e mede, por acks do cliente, a latência escrita->leitura.
//
// Uso:
//   s2-pipe-server [--duration 60] [--rate 30] [--out s2-pipe.json]
//     --rate 30  = pacing de 30 fps (padrão; caso real do produto)
//     --rate 0   = back-to-back (mede a vazão máxima do canal)
//
// O loop é request/response: envia o quadro e espera o ack do cliente. Se o
// cliente desconecta, o servidor avisa no console e espera novo cliente
// (recuperação — linha da tabela do S2).

use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows::Win32::System::Pipes::ConnectNamedPipe;
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForSingleObject};

use s2_pipe_server::pipe::{create_pipe, PIPE_NAME};
use s2_pipe_server::protocol::{
    read_ack, write_header, qpc_us, ACK_SIZE, FPS, FRAME_H, FRAME_W, FORMAT_NV12, HEADER_SIZE,
    MSG_SIZE, TICK_US,
};
use s2_pipe_server::stats::Stats;
use s2_pipe_server::frame;
use s2_pipe_server::sec;

// HANDLE não é `Send` por padrão no windows-rs, mas compartilhar o valor do
// handle entre threads é legítimo em Win32 (operações como CloseHandle são
// explícitamente thread-safe). Usado só para passar ao thread de conexão.
struct SendableHandle(windows::Win32::Foundation::HANDLE);
unsafe impl Send for SendableHandle {}

// Conecta o pipe e sinaliza o evento (roda em thread própria para não
// bloquear o deadline do processo).
fn wait_pipe_client(pipe: SendableHandle, ev: SendableHandle) {
    let _ = unsafe { ConnectNamedPipe(pipe.0, None) };
    let _ = unsafe { SetEvent(ev.0) };
}

struct Args {
    duration_s: u64,
    rate: u64, // 0 = back-to-back
    out: String,
}

fn parse_args() -> Args {
    let mut a = Args {
        duration_s: 60,
        rate: FPS,
        out: "s2-pipe.json".into(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(s) = it.next() {
        match s.as_str() {
            "--duration" => a.duration_s = it.next().and_then(|v| v.parse().ok()).unwrap_or(60),
            "--rate" => a.rate = it.next().and_then(|v| v.parse().ok()).unwrap_or(FPS),
            "--out" => a.out = it.next().unwrap_or_else(|| "s2-pipe.json".into()),
            _ => eprintln!("argumento ignorado: {s}"),
        }
    }
    a
}

fn run(args: &Args) -> Result<Stats, String> {
    let user_sid = sec::current_user_sid()?;
    // `_sd` (buffer da SD self-relative) precisa permanecer vivo enquanto `sa`
    // for usado; renomeado só para silenciar o aviso de "não lido".
    let (_sd, sa) = sec::pipe_security_attributes(&user_sid)?;

    println!("pipe: {PIPE_NAME}");
    println!(
        "DACL: acesso total (0x1F01FF) para {user_sid} + NT AUTHORITY\\LOCAL SERVICE (S-1-5-19)"
    );
    println!(
        "modo: {} fps{} por {} s",
        args.rate,
        if args.rate == 0 { " (back-to-back)" } else { "" },
        args.duration_s
    );

    let start = Instant::now();
    let mut st = Stats::default();
    let mut seq: u64 = 0;
    let mut msg = vec![0u8; MSG_SIZE];
    let mut ack = [0u8; ACK_SIZE];

    loop {
        // (Re)conexão do cliente. Com pipe síncrono, se o cliente abrir antes
        // do ConnectNamedPipe, a chamada retorna sucesso imediatamente.
        if start.elapsed() >= Duration::from_secs(args.duration_s) {
            break;
        }
        let hpipe = create_pipe(Some(&sa))?;
        st.connections += 1;
        println!("[+] aguardando cliente abrir o pipe ...");

        // Espera o cliente sem travar a thread principal: se ninguém conectar,
        // o deadline acima continua sendo respeitado (bug S2: ConnectNamedPipe
        // bloqueava para sempre e o processo só saía com Ctrl+C).
        let ev = unsafe { CreateEventW(None, true, false, None) }
            .map_err(|e| format!("CreateEventW: {e}"))?;
        let (sh, sev) = (SendableHandle(hpipe), SendableHandle(ev));
        let conn = std::thread::spawn(move || wait_pipe_client(sh, sev));

        // Espera o cliente na MESMA instância (não fecha/recria a cada
        // timeout: o pipe fica visível para o cliente até o deadline —
        // flapping close/recreate fazia o cliente nunca achar o pipe).
        let mut connected = false;
        loop {
            let w = unsafe { WaitForSingleObject(ev, 200) };
            if w == WAIT_OBJECT_0 {
                connected = true;
                break;
            }
            if start.elapsed() >= Duration::from_secs(args.duration_s) {
                break;
            }
        }
        let _ = unsafe { CloseHandle(ev) };
        if !connected {
            let _ = unsafe { CloseHandle(hpipe) }; // unblocka o ConnectNamedPipe
            let _ = conn.join();
            break; // deadline atingido
        }
        let _ = conn.join();
        println!("[+] cliente conectado");

        let conn_start = Instant::now();
        let mut frames_this_conn: u64 = 0;

        while start.elapsed() < Duration::from_secs(args.duration_s) {
            // Gera e envia o quadro (1 mensagem = header + NV12).
            seq += 1;
            let write_pts = qpc_us();
            frame::fill_nv12(&mut msg[HEADER_SIZE..], seq);
            write_header(&mut msg[..HEADER_SIZE], FRAME_W, FRAME_H, FORMAT_NV12, seq, write_pts);

            match unsafe { WriteFile(hpipe, Some(&msg), None, None) } {
                Ok(()) => st.frames_sent += 1,
                Err(e) => {
                    st.write_errors += 1;
                    println!("[!] WriteFile falhou (cliente saiu?): {e}");
                    break;
                }
            }
            st.bytes_sent += MSG_SIZE as u64;

            // Aguarda o ack e mede a latência escrita->leitura.
            match unsafe { ReadFile(hpipe, Some(&mut ack), None, None) } {
                Ok(()) => {
                    let (ack_seq, read_pts) = read_ack(&ack);
                    if ack_seq == seq && read_pts >= write_pts {
                        st.record_latency(read_pts - write_pts);
                    }
                    st.frames_acked += 1;
                }
                Err(e) => {
                    st.read_errors += 1;
                    println!("[!] ReadFile (ack) falhou: {e}");
                    break;
                }
            }

            // Pacing: alinha cada quadro ao tick N a partir da conexão.
            if args.rate > 0 {
                frames_this_conn += 1;
                let ideal = conn_start + Duration::from_micros(frames_this_conn * TICK_US);
                let now = Instant::now();
                if now < ideal {
                    std::thread::sleep(ideal - now);
                } else {
                    st.late += 1; // round-trip estourou o tick (atraso/backpressure)
                }
            }
        }

        let _ = unsafe { CloseHandle(hpipe) };
        if start.elapsed() >= Duration::from_secs(args.duration_s) {
            break;
        }
        println!("[~] cliente desconectado; aguardando novo cliente ...");
    }

    Ok(st)
}

fn main() {
    let args = parse_args();
    match run(&args) {
        Ok(st) => {
            let json = st.json("pipe", args.duration_s as f64);
            if let Err(e) = std::fs::write(&args.out, &json) {
                eprintln!("[!] não consegui gravar {}: {e}", args.out);
            }
            println!("\n=== RESUMO (pipe) ===\n{json}");
        }
        Err(e) => {
            eprintln!("[erro] {e}");
            std::process::exit(1);
        }
    }
}


