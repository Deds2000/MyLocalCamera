//! Fila limitada de comandos PC→celular por sessão.

use lc_protocol::{json::SetPayload, Message};
use tokio::sync::mpsc;

use super::SessionRegistry;

/// Comandos pendentes por sessão (SET e REQUEST_KEYFRAME).
pub(super) const COMMAND_QUEUE_CAPACITY: usize = 16;

/// Falha ao encaminhar um comando à sessão ativa.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SessionCommandError {
    #[error("sessão ativa não encontrada")]
    NoActiveSession,
    #[error("fila de comandos cheia")]
    QueueFull,
    #[error("canal da sessão encerrado")]
    Closed,
}

impl SessionRegistry {
    /// Enfileira pedido de keyframe sem esperar pelo socket.
    pub fn request_keyframe(
        &self,
        session_id: &str,
        stream: u8,
    ) -> Result<(), SessionCommandError> {
        self.send_command(session_id, Message::RequestKeyframe { stream })
    }

    /// Enfileira `SET` sem esperar pelo socket.
    pub fn send_set(
        &self,
        session_id: &str,
        payload: SetPayload,
    ) -> Result<(), SessionCommandError> {
        self.send_command(session_id, Message::Set(payload))
    }

    fn send_command(&self, session_id: &str, message: Message) -> Result<(), SessionCommandError> {
        let guard = self.current.lock().unwrap_or_else(|p| p.into_inner());
        let current = guard
            .as_ref()
            .filter(|session| session.session_id == session_id)
            .ok_or(SessionCommandError::NoActiveSession)?;
        current
            .commands
            .try_send(message)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => SessionCommandError::QueueFull,
                mpsc::error::TrySendError::Closed(_) => SessionCommandError::Closed,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionCommandError, COMMAND_QUEUE_CAPACITY};
    use crate::sessions::{Claim, SessionRegistry};
    use lc_protocol::Message;

    #[test]
    fn command_queue_is_bounded_and_delivers_in_order() {
        let registry = SessionRegistry::new();
        let mut commands = match registry.claim("session-1".to_owned(), "device-1", 1) {
            Claim::Granted { command_rx, .. } => command_rx,
            Claim::Busy => panic!("sessão deveria ser concedida"),
        };

        for stream in 0..COMMAND_QUEUE_CAPACITY {
            registry
                .request_keyframe("session-1", u8::try_from(stream).expect("stream"))
                .expect("comando cabe na fila");
        }
        assert_eq!(
            registry.request_keyframe("session-1", 1),
            Err(SessionCommandError::QueueFull)
        );
        for stream in 0..COMMAND_QUEUE_CAPACITY {
            assert_eq!(
                commands.try_recv().expect("comando em ordem"),
                Message::RequestKeyframe {
                    stream: u8::try_from(stream).expect("stream")
                }
            );
        }
    }
}
