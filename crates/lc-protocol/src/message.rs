//! `Message`: todas as mensagens do protocolo e conversão com `Frame`.

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::error::ProtocolError;
use crate::frame::{encode_frame, Frame};
use crate::json::{
    AudioHello, AudioTicket, Bye, Hello, Pending, SetPayload, State, Stats, StreamConfig,
    Welcome,
};
use crate::types::msg;

/// Limite do payload de HELLO: 64 KiB.
pub const MAX_HELLO_PAYLOAD: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Hello(Hello),
    Welcome(Welcome),
    AudioHello(AudioHello),
    Pending(Pending),
    AudioTicketRequest,
    AudioTicket(AudioTicket),
    Video {
        stream: u8,
        flags: u8,
        pts_us: u64,
        data: Bytes,
    },
    Audio {
        pts_us: u64,
        data: Bytes,
    },
    StreamConfig(StreamConfig),
    Set(SetPayload),
    State(State),
    Ping(u64),
    Pong(u64),
    Stats(Stats),
    RequestKeyframe {
        stream: u8,
    },
    Bye(Bye),
    /// Tipo de quadro não reconhecido: a conexão não cai por isso.
    Unknown {
        type_code: u8,
        len: usize,
    },
}

impl Message {
    /// Converte um quadro de rede em mensagem. Tipos desconhecidos viram `Unknown`.
    pub fn from_frame(f: &Frame) -> Result<Message, ProtocolError> {
        match f.msg_type {
            msg::HELLO => {
                if f.payload.len() > MAX_HELLO_PAYLOAD {
                    return Err(ProtocolError::TooLarge(f.payload.len().try_into().expect(
                        "tamanho de payload menor que u32::MAX",
                    )));
                }
                Ok(Self::Hello(decode_json(&f.payload)?))
            }
            msg::WELCOME => Ok(Self::Welcome(decode_json(&f.payload)?)),
            msg::AUDIO_HELLO => Ok(Self::AudioHello(decode_json(&f.payload)?)),
            msg::PENDING => Ok(Self::Pending(decode_json(&f.payload)?)),
            msg::AUDIO_TICKET_REQUEST => Ok(Self::AudioTicketRequest),
            msg::AUDIO_TICKET => Ok(Self::AudioTicket(decode_json(&f.payload)?)),
            msg::VIDEO => Ok(Self::Video {
                stream: f.stream,
                flags: f.flags,
                pts_us: f.pts_us,
                data: f.payload.clone(),
            }),
            msg::AUDIO => Ok(Self::Audio {
                pts_us: f.pts_us,
                data: f.payload.clone(),
            }),
            msg::STREAM_CONFIG => Ok(Self::StreamConfig(decode_json(&f.payload)?)),
            msg::SET => Ok(Self::Set(decode_json(&f.payload)?)),
            msg::STATE => Ok(Self::State(decode_json(&f.payload)?)),
            msg::PING => Ok(Self::Ping(read_u64(&f.payload, "PING")?)),
            msg::PONG => Ok(Self::Pong(read_u64(&f.payload, "PONG")?)),
            msg::STATS => Ok(Self::Stats(decode_json(&f.payload)?)),
            msg::REQUEST_KEYFRAME => Ok(Self::RequestKeyframe { stream: f.stream }),
            msg::BYE => Ok(Self::Bye(decode_json(&f.payload)?)),
            _ => Ok(Self::Unknown {
                type_code: f.msg_type,
                len: f.payload.len(),
            }),
        }
    }

    /// Serializa em bytes de quadro prontos para enviar.
    pub fn encode(&self, pts_us: u64) -> Result<Vec<u8>, ProtocolError> {
        match self {
            Self::Hello(m) => {
                let payload = json_bytes(m)?;
                if payload.len() > MAX_HELLO_PAYLOAD {
                    return Err(ProtocolError::TooLarge(
                        payload.len().try_into().expect("tamanho de payload menor que u32::MAX"),
                    ));
                }
                encode_frame(msg::HELLO, 0, 0, pts_us, &payload)
            }
            Self::Welcome(m) => encode_json(msg::WELCOME, 0, pts_us, m),
            Self::AudioHello(m) => encode_json(msg::AUDIO_HELLO, 0, pts_us, m),
            Self::Pending(m) => encode_json(msg::PENDING, 0, pts_us, m),
            Self::AudioTicketRequest => encode_frame(msg::AUDIO_TICKET_REQUEST, 0, 0, pts_us, &[]),
            Self::AudioTicket(m) => encode_json(msg::AUDIO_TICKET, 0, pts_us, m),
            Self::Video {
                stream,
                flags,
                data,
                ..
            } => encode_frame(msg::VIDEO, *flags, *stream, pts_us, data),
            Self::Audio { data, .. } => encode_frame(msg::AUDIO, 0, 3, pts_us, data),
            Self::StreamConfig(m) => encode_json(msg::STREAM_CONFIG, 0, pts_us, m),
            Self::Set(m) => encode_json(msg::SET, 0, pts_us, m),
            Self::State(m) => encode_json(msg::STATE, 0, pts_us, m),
            Self::Ping(v) => encode_frame(msg::PING, 0, 0, pts_us, &v.to_le_bytes()),
            Self::Pong(v) => encode_frame(msg::PONG, 0, 0, pts_us, &v.to_le_bytes()),
            Self::Stats(m) => encode_json(msg::STATS, 0, pts_us, m),
            Self::RequestKeyframe { stream } => {
                encode_frame(msg::REQUEST_KEYFRAME, 0, *stream, pts_us, &[])
            }
            Self::Bye(m) => encode_json(msg::BYE, 0, pts_us, m),
            Self::Unknown { type_code, .. } => Err(ProtocolError::Invalid(format!(
                "tipo desconhecido 0x{type_code:02X} não pode ser reenviado"
            ))),
        }
    }
}

/// Desserializa o payload JSON; erros viram `BadJson`.
fn decode_json<T: for<'de> Deserialize<'de>>(payload: &Bytes) -> Result<T, ProtocolError> {
    serde_json::from_slice(payload).map_err(|e| ProtocolError::BadJson(e.to_string()))
}

/// Payload PING/PONG: exatamente 8 bytes, u64 little-endian.
fn read_u64(payload: &Bytes, what: &str) -> Result<u64, ProtocolError> {
    if payload.len() != 8 {
        return Err(ProtocolError::Invalid(format!(
            "{what} deve ter payload de 8 bytes (recebi {})",
            payload.len()
        )));
    }
    // Invariante: `payload.len() == 8` foi verificado acima.
    Ok(u64::from_le_bytes(payload[..8].try_into().expect("fatia de 8 bytes")))
}

/// Serializa `value` para JSON e monta o quadro (`stream` fixo por chamada).
fn encode_json<T: Serialize>(
    msg_type: u8,
    stream: u8,
    pts_us: u64,
    value: &T,
) -> Result<Vec<u8>, ProtocolError> {
    let payload = json_bytes(value)?;
    encode_frame(msg_type, 0, stream, pts_us, &payload)
}

fn json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, ProtocolError> {
    serde_json::to_vec(value)
        .map_err(|e| ProtocolError::Invalid(format!("falha ao serializar JSON: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::string::string_regex;

    fn frame(msg_type: u8, stream: u8, flags: u8, payload: Vec<u8>) -> Frame {
        Frame {
            msg_type,
            flags,
            stream,
            pts_us: 0,
            payload: payload.into(),
        }
    }

    fn json_frame(msg_type: u8, json: &str) -> Frame {
        frame(msg_type, 0, 0, json.as_bytes().to_vec())
    }

    // UT-1.07: campos extras são ignorados.
    #[test]
    fn extra_fields_ignored() {
        let f = json_frame(
            msg::HELLO,
            r#"{
                "protocol": {"major": 1, "minor": 0},
                "app": {"version": "0.1.0", "platform": "android"},
                "device": {"id": "d1", "name": "P", "futuro": {"x": 1}},
                "auth": {"kind": "pair", "token": "abc"},
                "caps": {"cameras": [], "audio": true, "novo": 5}
            }"#,
        );
        assert!(matches!(
            Message::from_frame(&f),
            Ok(Message::Hello(_))
        ));
    }

    /// Assert que `from_frame` devolveu `BadJson(_)` e descarta a mensagem do erro.
    fn assert_bad_json(err: ProtocolError) {
        assert!(
            matches!(err, ProtocolError::BadJson(_)),
            "deveria ser BadJson: {err:?}"
        );
    }

    // UT-1.08: campos obrigatórios ausentes/tipo errado -> BadJson.
    #[test]
    fn missing_required_is_badjson() {
        let sem_auth = json_frame(
            msg::HELLO,
            r#"{
                "protocol": {"major": 1, "minor": 0},
                "app": {"version": "0.1.0", "platform": "android"},
                "device": {"id": "d1", "name": "P"}
            }"#,
        );
        assert_bad_json(Message::from_frame(&sem_auth).expect_err("deve falhar"));
        let sem_rev =
            json_frame(msg::STATE, r#"{"camera": {"mode": "single", "active": "back"}}"#);
        assert_bad_json(Message::from_frame(&sem_rev).expect_err("deve falhar"));
    }

    // UT-1.09: reason desconhecido vira `Reason::Other`.
    #[test]
    fn unknown_reason_is_other() {
        let f = json_frame(msg::WELCOME, r#"{"accepted": false, "reason": "algo_novo"}"#);
        match Message::from_frame(&f).expect("deve passar") {
            Message::Welcome(w) => {
                use crate::json::Reason;
                assert_eq!(w.reason, Some(Reason::Other));
            }
            other => panic!("esperado Welcome, veio {other:?}"),
        }
    }

    // UT-1.06 (parte): tipo desconhecido vira `Message::Unknown`.
    #[test]
    fn unknown_type_becomes_unknown() {
        let f = frame(0x99, 0, 0, vec![1, 2, 3]);
        assert_eq!(
            Message::from_frame(&f).expect("desconhecido não é erro"),
            Message::Unknown {
                type_code: 0x99,
                len: 3
            }
        );
    }

    #[test]
    fn hello_too_large() {
        let payload = vec![b'a'; 65_537];
        let f = frame(msg::HELLO, 0, 0, payload);
        assert_eq!(
            Message::from_frame(&f).expect_err("deve falhar"),
            ProtocolError::TooLarge(65_537)
        );
    }

    #[test]
    fn ping_wrong_size() {
        let f = frame(msg::PING, 0, 0, vec![1, 2, 3, 4, 5, 6, 7]);
        assert!(matches!(
            Message::from_frame(&f),
            Err(ProtocolError::Invalid(_))
        ));
    }

    // UT-1.22: ida e volta de Ping/Pong/Audio/RequestKeyframe/Bye com valores aleatórios.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn prop_message_roundtrip(
            pts in any::<u64>(),
            stream in any::<u8>(),
            data in prop::collection::vec(any::<u8>(), 0..=1024),
            bye_reason in string_regex("[a-z]{0,16}").unwrap(),
        ) {
            let data = Bytes::from(data);
            let messages = [
                Message::Ping(pts),
                Message::Pong(pts),
                Message::Audio { pts_us: pts, data: data.clone() },
                Message::RequestKeyframe { stream },
                Message::Bye(crate::json::Bye { reason: bye_reason }),
            ];
            for msg in messages {
                let bytes = msg.encode(pts).expect("encode válido");
                let mut reader = crate::frame::FrameReader::new();
                let frames = reader.push(&bytes).expect("leitura válida");
                prop_assert_eq!(frames.len(), 1);
                prop_assert_eq!(Message::from_frame(&frames[0]).expect("parse válido"), msg);
            }
        }
    }

}
