//! Tickets de áudio (`audioTicket`): 128 bits (32 hex), uso único,
//! validade de 10 s e presos ao `sessionId` de emissão.
//!
//! Módulo **puro**: o relógio é injetado como `now: u64` (segundos) —
//! nenhum `Instant`/relógio global. O ticket é segredo: comparado sempre
//! com [`ct_eq`] (tempo constante) e nunca exposto via `Debug`.

use std::collections::HashMap;
use std::fmt;

use crate::trust::secrets::{ct_eq, random_hex};

/// Validade de um ticket de áudio, em segundos.
pub const TICKET_TTL_SECS: u64 = 10;

/// Erros de resgate (`redeem`) de ticket de áudio.
#[derive(Debug, PartialEq, Eq)]
pub enum RedeemError {
    /// Ticket desconhecido (nunca emitido, já consumido ou invalidado).
    Unknown,
    /// Ticket válido de sessão, mas fora da validade.
    Expired,
    /// Ticket válido, mas emitido para outro `sessionId`.
    WrongSession,
}

/// Entrada de um ticket ativo (o ticket em si nunca aparece em `Debug`).
struct Entry {
    /// Ticket em hex minúsculo (32 chars) — segredo.
    ticket: String,
    /// Instante de emissão (`now` injetado, em segundos).
    issued_at: u64,
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field("ticket", &"<redigido>")
            .field("issued_at", &self.issued_at)
            .finish()
    }
}

/// Armazena os tickets de áudio ativos: um por sessão, uso único,
/// TTL de [`TICKET_TTL_SECS`] e presos à sessão de emissão.
///
/// Emitir um novo ticket para a mesma sessão (ex.: `AUDIO_TICKET_REQUEST`)
/// invalida o anterior ainda não usado.
pub struct TicketStore {
    /// `sessionId` → ticket ativo (no máximo um por sessão).
    by_session: HashMap<String, Entry>,
}

impl TicketStore {
    /// Cria o store vazio.
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_session: HashMap::new(),
        }
    }

    /// Emite um ticket para a sessão, invalidando qualquer ticket
    /// anterior ainda não usado da mesma sessão.
    pub fn issue(&mut self, session_id: &str, now: u64) -> String {
        let ticket = random_hex(16);
        self.by_session.insert(
            session_id.to_owned(),
            Entry {
                ticket: ticket.clone(),
                issued_at: now,
            },
        );
        ticket
    }

    /// Resgata o ticket se válido, **consumindo-o** (segundo uso falha).
    ///
    /// Erros: `Unknown` (ticket não existe mais), `WrongSession` (emitido
    /// para outra sessão) ou `Expired` (fora da validade; o ticket também
    /// é consumido, sem "renovação" por repetição).
    pub fn redeem(&mut self, session_id: &str, ticket: &str, now: u64) -> Result<(), RedeemError> {
        // Localiza a dona do ticket comparando **em tempo constante**
        // (sem `==` por caractere, que vazaría prefixes via timing).
        // Varre todas as entradas (sem saída antecipada); há no máximo
        // uma correspondência, pois cada ticket é único.
        let mut owner: Option<&str> = None;
        for (sess, entry) in &self.by_session {
            if ct_eq(&entry.ticket, ticket) {
                owner = Some(sess.as_str());
            }
        }
        let Some(owner) = owner else {
            return Err(RedeemError::Unknown);
        };
        if !ct_eq(owner, session_id) {
            return Err(RedeemError::WrongSession);
        }
        // `owner == session_id` verificado: remove a entrada e checa o TTL
        // (limite inclusivo: válido até `issued_at + TICKET_TTL_SECS`).
        let entry = match self.by_session.remove(session_id) {
            Some(entry) => entry,
            // Inatingível (owner verificado acima), mas sem panic.
            None => return Err(RedeemError::Unknown),
        };
        if now <= entry.issued_at + TICKET_TTL_SECS {
            Ok(())
        } else {
            Err(RedeemError::Expired)
        }
    }

    /// Remove o ticket de uma sessão encerrada (`session_id = Some(..)`)
    /// e todos os tickets expirados (além do TTL no instante `now`).
    pub fn purge(&mut self, session_id: Option<&str>, now: u64) {
        if let Some(session_id) = session_id {
            self.by_session.remove(session_id);
        }
        self.by_session
            .retain(|_, entry| now <= entry.issued_at + TICKET_TTL_SECS);
    }
}

impl Default for TicketStore {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for TicketStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Nunca expõe tickets (nem prefixos): só a contagem de sessões.
        f.debug_struct("TicketStore")
            .field("sessions", &self.by_session.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UT-1.19 — uso único: 1º `redeem` dá `Ok`, o 2º dá `Unknown`.
    #[test]
    fn ticket_single_use() {
        let mut store = TicketStore::new();
        let ticket = store.issue("s1", 100);
        assert_eq!(store.redeem("s1", &ticket, 100), Ok(()));
        assert_eq!(
            store.redeem("s1", &ticket, 100),
            Err(RedeemError::Unknown),
            "segundo uso deve falhar"
        );
    }

    /// UT-1.19 — TTL com limite inclusivo: `+10 s` válido, `+11 s` expirado.
    #[test]
    fn ticket_expires_after_ttl() {
        let mut store = TicketStore::new();
        let t1 = store.issue("s1", 100);
        assert_eq!(
            store.redeem("s1", &t1, 110),
            Ok(()),
            "+10 s é válido (limite inclusivo)"
        );
        let t2 = store.issue("s1", 100);
        assert_eq!(
            store.redeem("s1", &t2, 111),
            Err(RedeemError::Expired),
            "+11 s é expirado"
        );
    }

    /// UT-1.19 — o ticket é preso à sessão que o emitiu.
    #[test]
    fn ticket_bound_to_session() {
        let mut store = TicketStore::new();
        let ticket = store.issue("s1", 100);
        assert_eq!(
            store.redeem("s2", &ticket, 100),
            Err(RedeemError::WrongSession)
        );
        assert_eq!(
            store.redeem("s1", &ticket, 100),
            Ok(()),
            "tentativa de outra sessão não consome o ticket"
        );
    }

    /// UT-1.19 — emitir de novo invalida o ticket anterior não usado.
    #[test]
    fn new_issue_invalidates_previous() {
        let mut store = TicketStore::new();
        let old = store.issue("s1", 100);
        let fresh = store.issue("s1", 100);
        assert_ne!(old, fresh);
        assert_eq!(
            store.redeem("s1", &old, 100),
            Err(RedeemError::Unknown),
            "ticket anterior deve ser invalidado"
        );
        assert_eq!(store.redeem("s1", &fresh, 100), Ok(()));
    }

    /// `purge(Some(sess))` remove só a sessão encerrada;
    /// `purge(None, now)` remove os expirados.
    #[test]
    fn purge_removes_session_tickets() {
        let mut store = TicketStore::new();
        let t1 = store.issue("s1", 100);
        let t2 = store.issue("s2", 100);
        store.purge(Some("s1"), 100);
        assert_eq!(store.redeem("s1", &t1, 100), Err(RedeemError::Unknown));
        assert_eq!(
            store.redeem("s2", &t2, 100),
            Ok(()),
            "outras sessões não são afetadas"
        );

        let mut store2 = TicketStore::new();
        let ta = store2.issue("a", 100);
        let tb = store2.issue("b", 105);
        store2.purge(None, 111);
        assert_eq!(
            store2.redeem("a", &ta, 111),
            Err(RedeemError::Unknown),
            "expirado (100 + 10 < 111) deve ser removido"
        );
        assert_eq!(
            store2.redeem("b", &tb, 111),
            Ok(()),
            "válido até 115"
        );
    }

    /// `Debug` nunca contém o ticket.
    #[test]
    fn debug_hides_tickets() {
        let mut store = TicketStore::new();
        let ticket = store.issue("s1", 100);
        let rendered = format!("{store:?}");
        assert!(
            !rendered.contains(&ticket),
            "Debug expôs o ticket: {rendered}"
        );
        assert!(rendered.contains("TicketStore"));
    }
}
