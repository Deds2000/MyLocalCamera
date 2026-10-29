//! Fluxo de aprovação de pareamento: no máximo **um** pedido pendente.
//!
//! O handshake (`F1-T06`) abre o pedido com [`ApprovalManager::begin`],
//! envia `PENDING` ao celular e espera. A UI responde com
//! [`ApprovalManager::decide`] (via `ServerHandle::decide`); timeout e
//! desconexão do celular são resolvidos pelo próprio handshake, que
//! chama [`ApprovalManager::cancel`] e emite `ApprovalResolved`.
//!
//! `decide` para `request_id` desconhecido ou já resolvido é **ignorado**
//! (devolve `false`). `auto_approve` só vale em build de debug.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use tokio::sync::oneshot;

use crate::events::Decision;

/// Dados do pedido de aprovação (sem o `request_id`), para o evento
/// `ApprovalRequested`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalInfo {
    pub device_id: String,
    pub name: String,
    pub model: String,
    pub ip: String,
}

/// Erro de [`ApprovalManager::begin`]: já existe um pedido pendente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Busy;

impl std::fmt::Display for Busy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("já existe um pedido de aprovação pendente")
    }
}

impl std::error::Error for Busy {}

/// Pedido pendente (o oneshot carrega a decisão da UI).
#[derive(Debug)]
struct Pending {
    request_id: u64,
    tx: oneshot::Sender<Decision>,
}

/// Gerencia os pedidos de aprovação (no máximo um pendente).
#[derive(Debug, Default)]
pub struct ApprovalManager {
    next_id: AtomicU64,
    pending: Mutex<Option<Pending>>,
}

impl ApprovalManager {
    /// Cria o gerenciador sem pedidos pendentes.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Abre um novo pedido de aprovação.
    ///
    /// `info` é apenas contexto do chamador (o evento `ApprovalRequested`
    /// é emitido por ele); falha com [`Busy`] se já houver um pendente.
    pub fn begin(&self, _info: &ApprovalInfo) -> Result<(u64, oneshot::Receiver<Decision>), Busy> {
        let request_id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let (tx, rx) = oneshot::channel();
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        if pending.is_some() {
            return Err(Busy);
        }
        *pending = Some(Pending { request_id, tx });
        Ok((request_id, rx))
    }

    /// Decide o pedido pendente.
    ///
    /// Devolve `false` (e **ignora**) se o `request_id` é desconhecido ou
    /// já foi resolvido. Ao decidir, remove o pedido da fila.
    pub fn decide(&self, request_id: u64, d: Decision) -> bool {
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        match pending.as_mut() {
            Some(p) if p.request_id == request_id => {
                let p = pending.take().expect("pedido Some no match");
                p.tx.send(d).is_ok()
            }
            _ => false,
        }
    }

    /// Remove o pedido pendente sem decisão (celular desconectou, timeout
    /// estourou etc.); o `oneshot` morre e o awaiter do handshake vê o
    /// fim do canal. É um no-op se o pedido já não existe.
    pub fn cancel(&self, request_id: u64) {
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(p) = pending.as_ref() {
            if p.request_id == request_id {
                *pending = None;
            }
        }
    }
}

/// `auto_approve` só vale em build de **debug**; em release é ignorado.
/// (Em release o chamador loga o aviso — ver `handshake`.)
#[must_use]
pub fn effective_auto_approve(flag: bool, debug_build: bool) -> bool {
    flag && debug_build
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ST-3.06 — `decide` para `request_id` desconhecido é ignorado.
    #[test]
    fn decide_unknown_request_is_ignored() {
        let mgr = ApprovalManager::new();
        assert!(!mgr.decide(9999, Decision::ApproveAlways));
        // nada fica pendente
        let info = ApprovalInfo {
            device_id: "dev-1".to_owned(),
            name: "Fone".to_owned(),
            model: String::new(),
            ip: "127.0.0.1".to_owned(),
        };
        let (id, _rx) = mgr.begin(&info).expect("1º begin livre");
        assert!(!mgr.decide(9999, Decision::ApproveAlways), "id 9999 segue ignorado");
        assert!(mgr.decide(id, Decision::ApproveOnce), "id certo decide");
        assert!(
            !mgr.decide(id, Decision::Reject),
            "id já resolvido é ignorado"
        );
    }

    /// Dois pedidos simultâneos: o 2º é `Busy` até o 1º ser resolvido.
    #[test]
    fn at_most_one_pending_request() {
        let mgr = ApprovalManager::new();
        let info = ApprovalInfo {
            device_id: "dev-1".to_owned(),
            name: "Fone".to_owned(),
            model: String::new(),
            ip: "127.0.0.1".to_owned(),
        };
        let (id1, rx1) = mgr.begin(&info).expect("1º begin livre");
        assert!(mgr.begin(&info).is_err(), "2º begin é Busy");
        assert!(mgr.decide(id1, Decision::ApproveOnce), "decide libera");
        drop(rx1);
        let (id2, _rx2) = mgr.begin(&info).expect("begin volta a funcionar");
        assert_ne!(id1, id2);
    }

    /// ST-1.06 — `auto_approve` ignorado em release.
    #[test]
    fn auto_approve_only_in_debug() {
        assert!(!effective_auto_approve(true, false), "release: ignorado");
        assert!(effective_auto_approve(true, true), "debug: vale");
        assert!(!effective_auto_approve(false, true), "flag desligada: nada");
    }
}
