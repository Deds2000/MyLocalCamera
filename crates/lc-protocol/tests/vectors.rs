//! UT-1.01 / UT-1.10 — vetores oficiais de `frames` (docs/protocol-vectors.json).

use lc_protocol::{type_name, Frame, FrameReader, ProtocolError};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct VectorFile {
    frames: Vec<FrameVector>,
}

#[derive(Debug, Deserialize)]
struct FrameVector {
    name: String,
    hex: String,
    expect: Expect,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Expect {
    Ok(OkExpect),
    Err { error: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkExpect {
    frames: Vec<ExpectFrame>,
    pending_bytes: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpectFrame {
    type_code: u8,
    r#type: String,
    flags: u8,
    stream: u8,
    pts_us: String,
    payload_hex: String,
}

fn vector_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/protocol-vectors.json")
}

fn load() -> VectorFile {
    let raw = std::fs::read_to_string(vector_path()).expect("protocol-vectors.json presente");
    serde_json::from_str(&raw).expect("JSON válido")
}

/// O `hex` dos vetores contém espaços; `hex::decode` não aceita.
fn decode_hex(s: &str) -> Vec<u8> {
    let clean: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    hex::decode(&clean).expect("hex válido nos vetores")
}

fn expected_frames(ok: &OkExpect) -> Vec<Frame> {
    ok.frames
        .iter()
        .map(|f| {
            assert_eq!(
                type_name(f.type_code),
                f.r#type,
                "nome do tipo do código {}",
                f.type_code
            );
            Frame {
                msg_type: f.type_code,
                flags: f.flags,
                stream: f.stream,
                pts_us: f.pts_us.parse().expect("u64 nos vetores"),
                payload: decode_hex(&f.payload_hex).into(),
            }
        })
        .collect()
}

fn assert_err_name(err: &ProtocolError, name: &str) {
    assert!(
        format!("{err:?}").starts_with(name),
        "Debug {err:?} não começa com {name:?}"
    );
}

// UT-1.01: cada vetor nos 3 modos de entrega.
#[test]
fn vectors_frames_all_delivery_modes() {
    for v in &load().frames {
        let bytes = decode_hex(&v.hex);
        match &v.expect {
            Expect::Ok(ok) => {
                let expected = expected_frames(ok);
                let pending = ok.pending_bytes as usize;

                // modo 1: tudo em um único push
                let mut r = FrameReader::new();
                let frames = r.push(&bytes).expect("push inteiro válido");
                assert_eq!(frames, expected, "{}: push inteiro", v.name);
                assert_eq!(r.pending_len(), pending, "{}: push inteiro", v.name);

                // modo 2: byte a byte
                let mut r = FrameReader::new();
                let mut frames = Vec::new();
                for chunk in bytes.chunks(1) {
                    frames.extend(r.push(chunk).expect("byte a byte válido"));
                }
                assert_eq!(frames, expected, "{}: byte a byte", v.name);
                assert_eq!(r.pending_len(), pending, "{}: byte a byte", v.name);

                // modo 3: todas as divisões em 2 partes
                for i in 1..bytes.len() {
                    let mut r = FrameReader::new();
                    let mut frames = Vec::new();
                    frames.extend(r.push(&bytes[..i]).expect("divisão válida"));
                    frames.extend(r.push(&bytes[i..]).expect("divisão válida"));
                    assert_eq!(frames, expected, "{}: divisão em {i}", v.name);
                    assert_eq!(r.pending_len(), pending, "{}: divisão em {i}", v.name);
                }
            }
            Expect::Err { error } => {
                // modo 1: push inteiro deve errar
                let mut r = FrameReader::new();
                assert_err_name(&r.push(&bytes).expect_err("deve errar"), error);

                // modo 2: byte a byte — o erro aparece em algum push
                let mut r = FrameReader::new();
                let mut erro = None;
                for chunk in bytes.chunks(1) {
                    if let Err(e) = r.push(chunk) {
                        erro = Some(e);
                        break;
                    }
                }
                let Some(erro) = erro else {
                    panic!("{}: erro ausente no modo byte a byte", v.name)
                };
                assert_err_name(&erro, error);

                // modo 3: todas as divisões em 2 partes — o erro aparece em algum push
                for i in 1..bytes.len() {
                    let mut r = FrameReader::new();
                    let mut erro = None;
                    for part in [&bytes[..i], &bytes[i..]] {
                        if let Err(e) = r.push(part) {
                            erro = Some(e);
                            break;
                        }
                    }
                    let Some(erro) = erro else {
                        panic!("{}: divisão em {i} sem erro", v.name)
                    };
                    assert_err_name(&erro, error);
                }
            }
        }
    }
}

// UT-1.10: vetor `unknown_type_then_ping` em todas as divisões em 2 partes.
#[test]
fn fragment_every_position() {
    let file = load();
    let v = file
        .frames
        .iter()
        .find(|v| v.name == "unknown_type_then_ping")
        .expect("vetor unknown_type_then_ping presente");
    let bytes = decode_hex(&v.hex);
    let Expect::Ok(ok) = &v.expect else {
        panic!("vetor deveria ser Ok")
    };
    let expected = expected_frames(ok);
    assert_eq!(expected.len(), 2, "o vetor tem 2 quadros");
    for i in 1..bytes.len() {
        let mut r = FrameReader::new();
        let mut frames = Vec::new();
        frames.extend(r.push(&bytes[..i]).expect("divisão válida"));
        frames.extend(r.push(&bytes[i..]).expect("divisão válida"));
        assert_eq!(frames, expected, "divisão em {i}");
    }
}
