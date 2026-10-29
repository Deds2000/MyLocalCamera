//! Constantes do protocolo: tipos de mensagem, cabeçalho e limites.

/// Magic little-endian; na rede aparece como `43 4C`.
pub const MAGIC: u16 = 0x4C43;

/// Tamanho do cabeçalho de quadro, em bytes.
pub const HEADER_LEN: usize = 18;

/// Limite padrão de payload: 8 MiB.
pub const MAX_PAYLOAD: u32 = 8 * 1024 * 1024;

/// Códigos de tipo de mensagem (tabela do CHEATSHEET).
pub mod msg {
    pub const HELLO: u8 = 0x01;
    pub const WELCOME: u8 = 0x02;
    pub const AUDIO_HELLO: u8 = 0x03;
    pub const PENDING: u8 = 0x04;
    pub const AUDIO_TICKET_REQUEST: u8 = 0x05;
    pub const AUDIO_TICKET: u8 = 0x06;
    pub const VIDEO: u8 = 0x10;
    pub const AUDIO: u8 = 0x11;
    pub const STREAM_CONFIG: u8 = 0x12;
    pub const SET: u8 = 0x20;
    pub const STATE: u8 = 0x21;
    pub const PING: u8 = 0x30;
    pub const PONG: u8 = 0x31;
    pub const STATS: u8 = 0x32;
    pub const REQUEST_KEYFRAME: u8 = 0x40;
    pub const BYE: u8 = 0x7F;
}

/// Bits do campo `flags` do cabeçalho.
pub mod flags {
    /// bit0 = keyframe.
    pub const KEYFRAME: u8 = 1;
    /// bit1 = tem SPS/PPS.
    pub const HAS_PARAMS: u8 = 2;
}

/// Nome (em MAIÚSCULAS) do tipo de mensagem; `"UNKNOWN"` para códigos fora da tabela.
#[must_use]
pub fn type_name(code: u8) -> &'static str {
    match code {
        msg::HELLO => "HELLO",
        msg::WELCOME => "WELCOME",
        msg::AUDIO_HELLO => "AUDIO_HELLO",
        msg::PENDING => "PENDING",
        msg::AUDIO_TICKET_REQUEST => "AUDIO_TICKET_REQUEST",
        msg::AUDIO_TICKET => "AUDIO_TICKET",
        msg::VIDEO => "VIDEO",
        msg::AUDIO => "AUDIO",
        msg::STREAM_CONFIG => "STREAM_CONFIG",
        msg::SET => "SET",
        msg::STATE => "STATE",
        msg::PING => "PING",
        msg::PONG => "PONG",
        msg::STATS => "STATS",
        msg::REQUEST_KEYFRAME => "REQUEST_KEYFRAME",
        msg::BYE => "BYE",
        _ => "UNKNOWN",
    }
}
