//! Erros do protocolo.

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProtocolError {
    #[error("magic inválido")]
    BadMagic,
    #[error("payload grande demais: {0} bytes")]
    TooLarge(u32),
    #[error("json inválido: {0}")]
    BadJson(String),
    #[error("mensagem inválida: {0}")]
    Invalid(String),
}
