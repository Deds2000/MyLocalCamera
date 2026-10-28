//! Spike S2 — tipos e utilitários compartilhados (pipe-server e shm-server).
//!
//! Protocolo de quadro:
//! - `FrameHeader` (28 bytes, packed, little-endian) + payload NV12.
//! - `pts_us` é QPC em microssegundos (mesma máquina/mesmo relógio no cliente)
//!   → permite medir latência escrita→leitura.
//! - Pipe: cada mensagem começa com 4 bytes `kind` (1=frame, 2=stats).
//! - Shm: layout fixo `[header 28][nv12][stats 64]` + evento de sinalização.

pub mod sec;

use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

/// Largura padrão dos quadros sintéticos (1080p).
pub const WIDTH: u32 = 1920;
/// Altura padrão dos quadros sintéticos (1080p).
pub const HEIGHT: u32 = 1080;
/// Tamanho do payload NV12 1080p: w*h*3/2 = 3 110 400 bytes.
pub const NV12_SIZE: usize = (WIDTH as usize) * (HEIGHT as usize) * 3 / 2;
/// Tamanho do cabeçalho (packed, sem padding).
pub const HEADER_SIZE: usize = 28;
/// Tamanho do bloco de estatísticas do cliente.
pub const STATS_SIZE: usize = 64;

/// Tag de mensagem no pipe (4 bytes little-endian no início de cada mensagem).
pub const KIND_FRAME: u32 = 1;
pub const KIND_STATS: u32 = 2;

/// `MFVideoFormat_NV12` / KSDATAFORMAT_SUBTYPE_NV12 ("NV12" little-endian).
pub const FORMAT_NV12: u32 = 0x3231_564E;

/// Cabeçalho de quadro — espelhado em C++ (`SpikeFrameClient.h`) e no layout
/// da memória compartilhada. 28 bytes, sem padding (packed).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct FrameHeader {
    pub w: u32,
    pub h: u32,
    pub format: u32,
    pub seq: u64,
    pub pts_us: u64,
}

impl FrameHeader {
    pub fn new(seq: u64, pts_us: u64) -> Self {
        Self {
            w: WIDTH,
            h: HEIGHT,
            format: FORMAT_NV12,
            seq,
            pts_us,
        }
    }

    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let p = self as *const Self as *const u8;
        let mut out = [0u8; HEADER_SIZE];
        unsafe { core::ptr::copy_nonoverlapping(p, out.as_mut_ptr(), HEADER_SIZE) };
        out
    }

    pub fn from_bytes(b: &[u8]) -> Self {
        let mut out = [0u8; HEADER_SIZE];
        out.copy_from_slice(&b[..HEADER_SIZE]);
        unsafe { core::ptr::read_unaligned(out.as_ptr() as *const Self) }
    }

    /// `seq` no offset 12 (packed) — leitura não alinhada.
    pub fn seq(&self) -> u64 {
        unsafe {
            core::ptr::read_unaligned(
                (self as *const Self as *const u8).add(12) as *const u64,
            )
        }
    }

    /// `pts_us` no offset 20 (packed) — leitura não alinhada.
    pub fn pts_us(&self) -> u64 {
        unsafe {
            core::ptr::read_unaligned(
                (self as *const Self as *const u8).add(20) as *const u64,
            )
        }
    }
}

/// Estatísticas do cliente (DLL) — 64 bytes, packed.
/// Pipe: payload da mensagem `KIND_STATS`. Shm: bloco final da seção.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ClientStats {
    pub frames_read: u64,
    pub dropped: u64,
    pub lat_min_us: u64,
    pub lat_p50_us: u64,
    pub lat_p95_us: u64,
    pub lat_max_us: u64,
    pub last_seq: u64,
    pub last_read_us: u64,
}

impl ClientStats {
    pub fn to_bytes(&self) -> [u8; STATS_SIZE] {
        let p = self as *const Self as *const u8;
        let mut out = [0u8; STATS_SIZE];
        unsafe { core::ptr::copy_nonoverlapping(p, out.as_mut_ptr(), STATS_SIZE) };
        out
    }

    pub fn from_bytes(b: &[u8]) -> Self {
        let mut out = [0u8; STATS_SIZE];
        out.copy_from_slice(&b[..STATS_SIZE]);
        unsafe { core::ptr::read_unaligned(out.as_ptr() as *const Self) }
    }

    fn u64_at(off: usize, self_: *const Self) -> u64 {
        unsafe { core::ptr::read_unaligned((self_ as *const u8).add(off) as *const u64) }
    }

    /// Accessors não alinhados (struct packed; offsets em bytes).
    pub fn frames_read(&self) -> u64 { Self::u64_at(0, self as *const Self) }
    pub fn dropped(&self) -> u64 { Self::u64_at(8, self as *const Self) }
    pub fn lat_min_us(&self) -> u64 { Self::u64_at(16, self as *const Self) }
    pub fn lat_p50_us(&self) -> u64 { Self::u64_at(24, self as *const Self) }
    pub fn lat_p95_us(&self) -> u64 { Self::u64_at(32, self as *const Self) }
    pub fn lat_max_us(&self) -> u64 { Self::u64_at(40, self as *const Self) }
    pub fn last_seq(&self) -> u64 { Self::u64_at(48, self as *const Self) }
    pub fn last_read_us(&self) -> u64 { Self::u64_at(56, self as *const Self) }
}

/// Relógio de alta resolução (QPC) — mesmo relógio usado pela DLL.
#[derive(Clone, Copy)]
pub struct Qpc {
    freq: i64,
}

impl Qpc {
    pub fn new() -> Option<Self> {
        let mut freq: i64 = 0;
        let ok = unsafe { QueryPerformanceFrequency(&mut freq) }.is_ok();
        if ok && freq > 0 {
            Some(Self { freq })
        } else {
            None
        }
    }

    pub fn now_us(&self) -> u64 {
        let mut c: i64 = 0;
        let _ = unsafe { QueryPerformanceCounter(&mut c) };
        (c as u128 * 1_000_000 / self.freq as u128) as u64
    }
}

/// Gera um quadro NV12 sintético: Y = rampa diagonal que muda a cada `seq`,
/// UV = 128. O padrão visível se move a cada quadro (confirma transmissão).
pub fn fill_nv12_frame(buf: &mut [u8], seq: u64) {
    let w = WIDTH as usize;
    let h = HEIGHT as usize;
    let y_size = w * h;
    let mut y = 0;
    for row in 0..h {
        for x in 0..w {
            buf[y] = (x + row + (seq % 255) as usize) as u8;
            y += 1;
        }
    }
    for i in y_size..buf.len() {
        buf[i] = 128;
    }
}

/// Converte microssegundos em "x,xxx ms".
pub fn us_to_ms(us: u64) -> String {
    format!("{:.3}", us as f64 / 1000.0)
}

/// Nome wide com NUL incluído (o macro `w!` só aceita literais).
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(core::iter::once(0)).collect()
}
