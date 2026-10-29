//! F1-T02b — vetores oficiais de `json` (docs/protocol-vectors.json).
//!
//! Cada vetor é embrulhado em um `Frame` do tipo indicado e passa por
//! `Message::from_frame`; `expect == "ok"` exige sucesso **e ida e volta**
//! (encode + parse novamente = igual). `expect == "BadJson"` exige erro.

use lc_protocol::error::ProtocolError;
use lc_protocol::frame::{Frame, FrameReader};
use lc_protocol::json::Reason;
use lc_protocol::message::Message;
use lc_protocol::types::msg;
use serde::Deserialize;

#[derive(Deserialize)]
struct VectorFile {
    json: Vec<JsonVector>,
}

#[derive(Deserialize)]
struct JsonVector {
    name: String,
    message: String,
    json: serde_json::Value,
    expect: String,
}

fn vector_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/protocol-vectors.json")
}

fn load() -> Vec<JsonVector> {
    let raw = std::fs::read_to_string(vector_path()).expect("protocol-vectors.json presente");
    let file: VectorFile = serde_json::from_str(&raw).expect("JSON válido");
    file.json
}

fn msg_code(name: &str) -> u8 {
    match name {
        "HELLO" => msg::HELLO,
        "WELCOME" => msg::WELCOME,
        "AUDIO_HELLO" => msg::AUDIO_HELLO,
        "STATE" => msg::STATE,
        "SET" => msg::SET,
        "STREAM_CONFIG" => msg::STREAM_CONFIG,
        "STATS" => msg::STATS,
        "BYE" => msg::BYE,
        other => panic!("tipo de vetor não tratado: {other}"),
    }
}

fn roundtrip(message: &Message, name: &str) -> Message {
    let bytes = message.encode(0).expect("encode válido");
    let mut reader = FrameReader::new();
    let frames = reader.push(&bytes).expect("leitura válida");
    assert_eq!(frames.len(), 1, "um quadro único em {name}");
    Message::from_frame(&frames[0]).expect("parse do quadro codificado")
}

#[test]
fn vectors_json_all() {
    for v in load() {
        let payload = serde_json::to_vec(&v.json).expect("serializar vetor");
        let f = Frame {
            msg_type: msg_code(&v.message),
            flags: 0,
            stream: 0,
            pts_us: 0,
            payload: payload.into(),
        };
        match v.expect.as_str() {
            "ok" => {
                let m = Message::from_frame(&f)
                    .unwrap_or_else(|e| panic!("vetor {} deveria ser Ok: {e}", v.name));
                assert_eq!(
                    roundtrip(&m, &v.name),
                    m,
                    "ida e volta do vetor {}",
                    v.name
                );
                // UT-1.09: reason desconhecido é `Other` (não erro).
                if v.name == "welcome_unknown_reason" {
                    match &m {
                        Message::Welcome(w) => assert_eq!(w.reason, Some(Reason::Other)),
                        other => panic!("esperado Welcome, veio {other:?}"),
                    }
                }
            }
            "BadJson" => match Message::from_frame(&f) {
                Err(ProtocolError::BadJson(_)) => {}
                other => panic!("vetor {} deveria ser BadJson, veio {other:?}", v.name),
            },
            other => panic!("expect desconhecido: {other:?} (vetor {})", v.name),
        }
    }
}
