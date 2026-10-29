//! Leitor incremental de quadros e writer (cabeçalho de 18 bytes, little-endian).
//!
//! Layout do cabeçalho: magic u16, type u8, flags u8, stream u8, reserved u8 (0),
//! ptsUs u64, length u32.

use bytes::{Buf, Bytes, BytesMut};

use crate::{error::ProtocolError, types::*};

/// Um quadro completo lido da rede.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub msg_type: u8,
    pub flags: u8,
    pub stream: u8,
    pub pts_us: u64,
    pub payload: Bytes,
}

/// Leitor incremental: aceita bytes fragmentados em várias chamadas a `push`.
///
/// Depois de um erro (`BadMagic`/`TooLarge`) o leitor fica **envenenado**:
/// toda chamada seguinte a `push` devolve o mesmo erro e o buffer é descartado
/// (o chamador deve fechar a conexão).
pub struct FrameReader {
    buf: BytesMut,
    max_payload: u32,
    poisoned: Option<ProtocolError>,
}

impl Default for FrameReader {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameReader {
    /// Cria um leitor com o limite padrão de payload (8 MiB).
    #[must_use]
    pub fn new() -> Self {
        Self::with_max_payload(MAX_PAYLOAD)
    }

    /// Cria um leitor com um limite de payload personalizado.
    #[must_use]
    pub fn with_max_payload(max_payload: u32) -> Self {
        Self {
            buf: BytesMut::new(),
            max_payload,
            poisoned: None,
        }
    }

    /// Bytes ainda não consumidos (cabeçalho/payload incompletos).
    #[must_use]
    pub fn pending_len(&self) -> usize {
        self.buf.len()
    }

    /// Alimenta bytes e devolve todos os quadros completos.
    pub fn push(&mut self, data: &[u8]) -> Result<Vec<Frame>, ProtocolError> {
        if let Some(err) = &self.poisoned {
            return Err(err.clone());
        }
        self.buf.extend_from_slice(data);
        let mut frames = Vec::new();
        loop {
            // Invariante: o loop só continua com `buf.len() >= HEADER_LEN`,
            // então as fatias de cabeçalho abaixo sempre existem.
            if self.buf.len() < HEADER_LEN {
                break;
            }
            let magic =
                u16::from_le_bytes(self.buf[0..2].try_into().expect("slice de tamanho fixo"));
            if magic != MAGIC {
                return self.poison(ProtocolError::BadMagic);
            }
            let msg_type = self.buf[2];
            let flags = self.buf[3];
            let stream = self.buf[4];
            let pts_us =
                u64::from_le_bytes(self.buf[6..14].try_into().expect("slice de tamanho fixo"));
            let length =
                u32::from_le_bytes(self.buf[14..18].try_into().expect("slice de tamanho fixo"));
            // Valida `length` ANTES de reservar/consumir qualquer payload.
            if length > self.max_payload {
                return self.poison(ProtocolError::TooLarge(length));
            }
            // Invariante: `length <= max_payload` (u32), cabe em usize.
            let total = HEADER_LEN + usize::try_from(length).expect("u32 cabe em usize");
            if self.buf.len() < total {
                break; // payload incompleto: espera mais bytes
            }
            self.buf.advance(HEADER_LEN); // consome o cabeçalho
            let payload = self.buf.split_to(length as usize).freeze();
            frames.push(Frame {
                msg_type,
                flags,
                stream,
                pts_us,
                payload,
            });
        }
        Ok(frames)
    }

    /// Envenena o leitor: guarda o erro e descarta o buffer.
    fn poison(&mut self, err: ProtocolError) -> Result<Vec<Frame>, ProtocolError> {
        self.poisoned = Some(err.clone());
        self.buf.clear();
        Err(err)
    }
}

/// Serializa um quadro. Erro se payload > `MAX_PAYLOAD`.
pub fn encode_frame(
    msg_type: u8,
    flags: u8,
    stream: u8,
    pts_us: u64,
    payload: &[u8],
) -> Result<Vec<u8>, ProtocolError> {
    // Invariante: um `Vec` com mais de u32::MAX bytes é irreal; o limite
    // de 8 MiB torna o `expect` inatingível.
    let len = u32::try_from(payload.len()).expect("payload menor que u32::MAX");
    if len > MAX_PAYLOAD {
        return Err(ProtocolError::TooLarge(len));
    }
    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.push(msg_type);
    out.push(flags);
    out.push(stream);
    out.push(0); // reserved
    out.extend_from_slice(&pts_us.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(payload);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn roundtrip_frame(frame: &Frame) -> Frame {
        let bytes = encode_frame(
            frame.msg_type,
            frame.flags,
            frame.stream,
            frame.pts_us,
            &frame.payload,
        )
        .expect("encode válido");
        let mut reader = FrameReader::new();
        let frames = reader.push(&bytes).expect("leitura válida");
        assert_eq!(reader.pending_len(), 0);
        assert_eq!(frames, vec![frame.clone()]);
        frames.into_iter().next().expect("um quadro")
    }

    // UT-1.03
    #[test]
    fn roundtrip_pts_extremes() {
        for pts in [0u64, u64::MAX] {
            for payload in [Vec::new(), vec![0xABu8; 1024]] {
                let frame = Frame {
                    msg_type: msg::VIDEO,
                    flags: flags::KEYFRAME | flags::HAS_PARAMS,
                    stream: 1,
                    pts_us: pts,
                    payload: payload.into(),
                };
                assert_eq!(roundtrip_frame(&frame), frame);
            }
        }
    }

    // UT-1.04
    #[test]
    fn bad_magic_errors() {
        let mut bytes = encode_frame(msg::PING, 0, 0, 0, &[1, 2, 3, 4, 5, 6, 7, 8]).expect("ok");
        bytes[0] = 0x58; // corrompe o magic
        let mut reader = FrameReader::new();
        assert_eq!(
            reader.push(&bytes).expect_err("deve falhar"),
            ProtocolError::BadMagic
        );
    }

    // UT-1.05
    #[test]
    fn too_large_before_alloc() {
        let max: u32 = 8;
        let mut reader = FrameReader::with_max_payload(max);
        let mut header = vec![0u8; HEADER_LEN];
        header[0..2].copy_from_slice(&MAGIC.to_le_bytes());
        header[14..18].copy_from_slice(&(max + 1).to_le_bytes()); // length = max + 1
        assert_eq!(
            reader.push(&header).expect_err("deve falhar"),
            ProtocolError::TooLarge(max + 1)
        );
        assert!(reader.pending_len() <= HEADER_LEN);
    }

    // UT-1.11
    #[test]
    fn glued_frames() {
        let b1 = encode_frame(msg::PING, 0, 0, 1, &[1, 2]).expect("ok");
        let b2 = encode_frame(msg::PONG, 0, 0, 2, b"ola").expect("ok");
        let b3 = encode_frame(msg::BYE, 0, 3, 3, &[]).expect("ok");
        let mut all = Vec::with_capacity(b1.len() + b2.len() + b3.len());
        all.extend_from_slice(&b1);
        all.extend_from_slice(&b2);
        all.extend_from_slice(&b3);
        let mut reader = FrameReader::new();
        let frames = reader.push(&all).expect("ok");
        assert_eq!(frames.len(), 3);
        assert_eq!(
            frames
                .iter()
                .map(|f| (f.msg_type, f.pts_us))
                .collect::<Vec<_>>(),
            [(msg::PING, 1u64), (msg::PONG, 2), (msg::BYE, 3)]
        );
        assert_eq!(reader.pending_len(), 0);
    }

    #[test]
    fn encode_frame_too_large() {
        let payload = vec![0u8; (MAX_PAYLOAD + 1) as usize];
        assert_eq!(
            encode_frame(msg::VIDEO, 0, 1, 0, &payload).expect_err("deve falhar"),
            ProtocolError::TooLarge(MAX_PAYLOAD + 1)
        );
    }

    #[test]
    fn poisoned_reader_keeps_error() {
        let mut reader = FrameReader::new();
        let mut bad = vec![0u8; HEADER_LEN];
        bad[0] = 0x58; // magic inválido
        let err = reader.push(&bad).expect_err("deve falhar");
        assert_eq!(err, ProtocolError::BadMagic);
        // continua envenenado mesmo recebindo bytes válidos (ou vazio)
        assert_eq!(reader.push(&[]).expect_err("envenenado"), err);
        assert_eq!(
            reader.push(&MAGIC.to_le_bytes()).expect_err("envenenado"),
            err
        );
        assert_eq!(reader.pending_len(), 0);
    }

    fn any_frame() -> impl Strategy<Value = Frame> {
        (
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u64>(),
            prop::collection::vec(any::<u8>(), 0..=2048),
        )
            .prop_map(|(msg_type, flags, stream, pts_us, payload)| Frame {
                msg_type,
                flags,
                stream,
                pts_us,
                payload: payload.into(),
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        // UT-1.20: qualquer particionamento em blocos de 1..=64 bytes preserva os quadros.
        #[test]
        fn prop_any_split_same_frames(
            frames in prop::collection::vec(any_frame(), 0..=20),
            blocks in prop::collection::vec(any::<u8>().prop_map(|v| 1usize + (v % 64) as usize), 0..=800),
        ) {
            let mut bytes = Vec::new();
            for f in &frames {
                bytes.extend(encode_frame(f.msg_type, f.flags, f.stream, f.pts_us, &f.payload)
                    .expect("encode válido"));
            }
            let mut reader = FrameReader::new();
            let mut out = Vec::new();
            let mut pos = 0;
            let mut it = blocks.into_iter();
            while pos < bytes.len() {
                // se os blocos acabarem, 1 ainda é um tamanho válido (1..=64)
                let size = it.next().unwrap_or(1);
                let end = (pos + size).min(bytes.len());
                out.extend(reader.push(&bytes[pos..end]).expect("leitura válida"));
                pos = end;
            }
            assert_eq!(out, frames);
            assert_eq!(reader.pending_len(), 0);
        }

        // UT-1.21: bytes aleatórios nunca causam panic; o buffer pendente fica limitado.
        #[test]
        fn prop_random_bytes_never_panic(
            data in prop::collection::vec(any::<u8>(), 0..=4096),
            blocks in prop::collection::vec(any::<u8>().prop_map(|v| 1usize + (v % 100) as usize), 0..=64),
        ) {
            let max_payload: u32 = 4096;
            let mut reader = FrameReader::with_max_payload(max_payload);
            let limit = HEADER_LEN + max_payload as usize;
            let mut pos = 0;
            let mut it = blocks.into_iter();
            while pos < data.len() {
                let size = it.next().unwrap_or(1);
                let end = (pos + size).min(data.len());
                // Err (BadMagic/TooLarge) é aceitável; o importante: sem panic.
                let _ = reader.push(&data[pos..end]);
                pos = end;
                assert!(reader.pending_len() <= limit, "pending {} > {limit}", reader.pending_len());
            }
        }
    }
}
