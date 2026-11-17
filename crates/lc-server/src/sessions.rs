//! Sessão ativa do PC: **uma única** sessão de cada vez.
//!
//! O registro vive em `Mutex<Option<ActiveSession>>` (std: as operações
//! são curtas e nunca `await` sob o lock). A substituição de sessão
//! (mesmo `deviceId`) sinaliza a antiga via `oneshot` — quem é chutado
//! envia `BYE{reason:"replaced"}` e fecha com [`crate::events::CloseReason::Replaced`].

use std::sync::{Arc, Mutex};

use lc_protocol::Message;
use tokio::sync::{mpsc, oneshot};

mod commands;
mod counters;
pub use commands::SessionCommandError;
use commands::COMMAND_QUEUE_CAPACITY;
pub use counters::SessionCounters;

/// Sessão ativa.
#[derive(Debug)]
pub struct ActiveSession {
    /// `sessionId` emitido no `WELCOME` (16 bytes em hex).
    pub session_id: String,
    /// Dispositivo dono da sessão.
    pub device_id: String,
    /// Conexão que a originou.
    pub conn_id: u64,
    /// Sinal de substituição (o winner do `claim` guarda o receptor).
    pub kick: oneshot::Sender<()>,
    /// Contadores da sessão (o F1-T10 os lê via
    /// [`SessionRegistry::counters`]).
    pub counters: Arc<SessionCounters>,
    /// Sinal do canal de áudio ativo (dispara ao substituir o áudio —
    /// ele fecha com `Replaced` — ou ao liberar a sessão — ele fecha
    /// com `PeerClosed`). `None` sem áudio aberto.
    pub audio_kick: Option<oneshot::Sender<()>>,
    /// Único produtor da fila PC→celular; o laço da sessão é o escritor.
    pub commands: mpsc::Sender<Message>,
}

/// Resultado de [`SessionRegistry::claim`].
#[derive(Debug)]
pub enum Claim {
    /// Sessão ativa concedida.
    ///
    /// `kicked`: `true` se havia sessão ativa do **mesmo** `deviceId`
    /// (ela foi substituída). `kick_rx`: dispara se **esta** sessão for
    /// substituída por outra conexão do mesmo dispositivo.
    Granted {
        kicked: bool,
        kick_rx: oneshot::Receiver<()>,
        command_rx: mpsc::Receiver<Message>,
    },
    /// Há sessão ativa de **outro** `deviceId`.
    Busy,
}

/// Registro da sessão ativa (no máximo uma).
#[derive(Debug, Default)]
pub struct SessionRegistry {
    current: Mutex<Option<ActiveSession>>,
}

impl SessionRegistry {
    /// Cria o registro vazio.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: Mutex::new(None),
        }
    }

    /// Concede a sessão ativa para `device_id`.
    ///
    /// - Sem sessão ativa → concede.
    /// - Sessão ativa do **mesmo** `deviceId` → substitui: o `kick` da
    ///   antiga dispara (ela envia `BYE` e fecha com `Replaced`).
    /// - Sessão ativa de **outro** `deviceId` → [`Claim::Busy`].
    pub fn claim(&self, session_id: String, device_id: &str, conn_id: u64) -> Claim {
        let (kick_tx, kick_rx) = oneshot::channel();
        let mut guard = self.current.lock().unwrap_or_else(|p| p.into_inner());
        match guard.as_ref() {
            None => {
                let (command_tx, command_rx) = mpsc::channel(COMMAND_QUEUE_CAPACITY);
                *guard = Some(Self::entry(
                    session_id, device_id, conn_id, kick_tx, command_tx,
                ));
                Claim::Granted {
                    kicked: false,
                    kick_rx,
                    command_rx,
                }
            }
            Some(old) if old.device_id == device_id => {
                let old = guard
                    .take()
                    .expect("verificado com `as_ref` acima, sob o mesmo lock");
                // `send` do oneshot do tokio é síncrono; o sender antigo
                // é descartado após o disparo.
                let _ = old.kick.send(());
                // O áudio da sessão antiga também fecha (a sessão que o
                // donava não é mais a ativa; ele lê `PeerClosed`).
                if let Some(audio) = old.audio_kick {
                    let _ = audio.send(());
                }
                let (command_tx, command_rx) = mpsc::channel(COMMAND_QUEUE_CAPACITY);
                *guard = Some(Self::entry(
                    session_id, device_id, conn_id, kick_tx, command_tx,
                ));
                Claim::Granted {
                    kicked: true,
                    kick_rx,
                    command_rx,
                }
            }
            // Outro dispositivo: a sessão ativa **não** é tocada.
            Some(_) => Claim::Busy,
        }
    }

    /// Libera a sessão ao encerrar — só se ela ainda é a ativa (a sessão
    /// antiga, já substituída, não pode derrubar a nova). Dispara o kick
    /// do áudio da sessão (o task de áudio vê a sessão fora da ativa e
    /// fecha com `PeerClosed`).
    pub fn release(&self, session_id: &str) {
        let mut guard = self.current.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(current) = guard.as_ref() {
            if current.session_id == session_id {
                let current = guard
                    .take()
                    .expect("verificado com `as_ref` acima, sob o mesmo lock");
                if let Some(audio) = current.audio_kick {
                    let _ = audio.send(());
                }
                *guard = None;
            }
        }
    }

    /// Registra o canal de áudio da sessão — só se `session_id` ainda é
    /// a ativa. Substitui um canal anterior (dispara o kick dele → ele
    /// fecha com `Replaced`).
    ///
    /// Devolve os contadores da sessão na **mesma** seção crítica
    /// (evita race "a sessão morreu entre registrar e ler os
    /// contadores"); `None` se a sessão não é mais a ativa (o task de
    /// áudio deve encerrar sem abrir o canal).
    pub fn set_audio(
        &self,
        session_id: &str,
        kick_tx: oneshot::Sender<()>,
    ) -> Option<Arc<SessionCounters>> {
        let mut guard = self.current.lock().unwrap_or_else(|p| p.into_inner());
        match guard.as_mut() {
            Some(cur) if cur.session_id == session_id => {
                if let Some(old) = cur.audio_kick.take() {
                    let _ = old.send(());
                }
                cur.audio_kick = Some(kick_tx);
                Some(cur.counters.clone())
            }
            _ => None,
        }
    }

    /// `true` se `session_id` ainda é a sessão ativa.
    #[must_use]
    pub fn is_active(&self, session_id: &str) -> bool {
        matches!(
            self.current
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .as_ref(),
            Some(cur) if cur.session_id == session_id
        )
    }

    /// `Arc` dos contadores da sessão ativa (ou `None` se `session_id`
    /// não é mais a ativa).
    #[must_use]
    pub fn counters(&self, session_id: &str) -> Option<Arc<SessionCounters>> {
        self.current
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .and_then(|cur| (cur.session_id == session_id).then(|| cur.counters.clone()))
    }

    fn entry(
        session_id: String,
        device_id: &str,
        conn_id: u64,
        kick: oneshot::Sender<()>,
        commands: mpsc::Sender<Message>,
    ) -> ActiveSession {
        ActiveSession {
            session_id,
            device_id: device_id.to_owned(),
            conn_id,
            kick,
            counters: Arc::new(SessionCounters::default()),
            audio_kick: None,
            commands,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mesmo dispositivo substitui; outro fica `Busy`; a release da
    /// sessão antiga não afeta a nova.
    #[test]
    fn claim_same_device_replaces_others_busy() {
        let reg = SessionRegistry::new();

        let first = reg.claim("s1".to_owned(), "dev-1", 1);
        assert!(
            matches!(first, Claim::Granted { kicked: false, .. }),
            "sem sessão ativa deve conceder"
        );

        assert!(
            matches!(reg.claim("s2".to_owned(), "dev-2", 2), Claim::Busy),
            "outro deviceId deve ficar busy"
        );

        let kicked = match reg.claim("s2".to_owned(), "dev-1", 2) {
            Claim::Granted { kicked, .. } => kicked,
            Claim::Busy => panic!("mesmo deviceId não pode ficar busy"),
        };
        assert!(kicked, "mesmo deviceId substitui");

        // O kick da primeira sessão disparou (receiver dela já recebeu).
        // A primeira sessão encerra: release com `s1` é no-op...
        reg.release("s1");
        // ...e a segunda segue ativa (claim de novo dispositivo → busy).
        assert!(matches!(
            reg.claim("s3".to_owned(), "dev-9", 3),
            Claim::Busy
        ));
        // Release da segunda libera.
        reg.release("s2");
        assert!(matches!(
            reg.claim("s4".to_owned(), "dev-9", 4),
            Claim::Granted { .. }
        ));
    }

    /// `kick_rx` do winner recebe quando ele é substituído.
    #[test]
    fn kick_fires_on_replacement() {
        let reg = SessionRegistry::new();
        let mut first = match reg.claim("s1".to_owned(), "dev-1", 1) {
            Claim::Granted {
                kicked, kick_rx, ..
            } => {
                assert!(!kicked);
                kick_rx
            }
            _ => panic!(),
        };
        reg.claim("s2".to_owned(), "dev-1", 2);
        // `try_recv` (síncrono, sem runtime): valor pendente = kick disparou.
        assert!(first.try_recv().is_ok(), "kick da substituída disparou");
    }
}
