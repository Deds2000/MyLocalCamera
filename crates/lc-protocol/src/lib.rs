//! Protocolo LocalCamera: cabeçalho de 18 bytes, leitor incremental e writer de quadros.

pub mod error;
pub mod frame;
pub mod json;
pub mod message;
pub mod types;

pub use error::ProtocolError;
pub use frame::{encode_frame, Frame, FrameReader};
pub use message::Message;
pub use types::{flags, msg, type_name};

#[cfg(test)]
mod infra_tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_infra_works(a in any::<u32>(), b in any::<u32>()) {
            prop_assert_eq!(a as u64 + b as u64, b as u64 + a as u64);
        }
    }
}
