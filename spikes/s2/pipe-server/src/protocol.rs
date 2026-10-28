// Protocolo do spike S2 (compartilhado por pipe-server, smoke-client e pela
// DLL em spikes/s2/dll). Ver docs/tasks/fase-0/F0-S2.md e docs/spikes/S2.md.
//
// Mensagem de quadro (little-endian na linha):
//   header 28 bytes: w:u32, h:u32, format:u32 (1 = NV12), seq:u64, pts_us:u64
//   payload: NV12 w*h*3/2 bytes
//
// Mensagem de ack (cliente -> servidor, 16 bytes): seq:u64, read_pts_us:u64.
//
// pts_us / read_pts_us usam o QPC do Windows (QueryPerformanceCounter), que é
// uma contagem global da máquina: os valores de processos diferentes são
// comparáveis, então a latência escrita->leitura = read_pts_us - pts_us.

use std::sync::OnceLock;

use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

pub const FORMAT_NV12: u32 = 1;
pub const HEADER_SIZE: usize = 4 + 4 + 4 + 8 + 8; // 28
pub const FRAME_W: u32 = 1920;
pub const FRAME_H: u32 = 1080;
pub const NV12_SIZE: usize = (FRAME_W as usize) * (FRAME_H as usize) * 3 / 2; // 3 110 400
pub const MSG_SIZE: usize = HEADER_SIZE + NV12_SIZE; // 3 110 428
pub const ACK_SIZE: usize = 16;
pub const FPS: u64 = 30;
pub const TICK_US: u64 = 1_000_000 / FPS; // 33 333

#[derive(Clone, Copy, Debug, Default)]
pub struct FrameHeader {
    pub w: u32,
    pub h: u32,
    pub format: u32,
    pub seq: u64,
    pub pts_us: u64,
}

pub fn write_header(buf: &mut [u8], w: u32, h: u32, format: u32, seq: u64, pts_us: u64) {
    debug_assert_eq!(buf.len(), HEADER_SIZE);
    buf[0..4].copy_from_slice(&w.to_le_bytes());
    buf[4..8].copy_from_slice(&h.to_le_bytes());
    buf[8..12].copy_from_slice(&format.to_le_bytes());
    buf[12..20].copy_from_slice(&seq.to_le_bytes());
    buf[20..28].copy_from_slice(&pts_us.to_le_bytes());
}

pub fn read_header(buf: &[u8]) -> FrameHeader {
    debug_assert!(buf.len() >= HEADER_SIZE);
    FrameHeader {
        w: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
        h: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
        format: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
        seq: u64::from_le_bytes(buf[12..20].try_into().unwrap()),
        pts_us: u64::from_le_bytes(buf[20..28].try_into().unwrap()),
    }
}

pub fn write_ack(buf: &mut [u8], seq: u64, read_pts_us: u64) {
    debug_assert_eq!(buf.len(), ACK_SIZE);
    buf[0..8].copy_from_slice(&seq.to_le_bytes());
    buf[8..16].copy_from_slice(&read_pts_us.to_le_bytes());
}

pub fn read_ack(buf: &[u8]) -> (u64, u64) {
    (
        u64::from_le_bytes(buf[0..8].try_into().unwrap()),
        u64::from_le_bytes(buf[8..16].try_into().unwrap()),
    )
}

fn qpc_freq() -> &'static i64 {
    static FREQ: OnceLock<i64> = OnceLock::new();
    FREQ.get_or_init(|| {
        let mut f: i64 = 0;
        unsafe { QueryPerformanceFrequency(&mut f) }.unwrap_or_else(|e| panic!("QPC freq: {e}"));
        assert!(f > 0);
        f
    })
}

/// Tempo atual em microssegundos (QPC, comparável entre processos da máquina).
pub fn qpc_us() -> u64 {
    let mut c: i64 = 0;
    unsafe { QueryPerformanceCounter(&mut c) }.unwrap_or_else(|e| panic!("QPC: {e}"));
    ((c as u128) * 1_000_000 / *qpc_freq() as u128) as u64
}
