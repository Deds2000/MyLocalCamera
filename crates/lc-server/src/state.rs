//! Máquina de estados da conexão.

/// Estados possíveis de uma conexão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnState {
    /// Aguardando o primeiro quadro (`HELLO`).
    AwaitHello,
    /// Handshake em curso (após `HELLO` válido).
    Authenticating,
    /// Esperando aprovação no PC.
    Pending,
    /// Autenticada e em uso.
    Active,
    /// Fechando.
    Closing,
}

/// Transição inválida na máquina de estados.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("transição inválida: {from:?} -> {to:?}")]
pub struct InvalidTransition {
    /// Estado de origem.
    pub from: ConnState,
    /// Estado de destino.
    pub to: ConnState,
}

/// Valida e aplica uma transição de estado.
///
/// Válidas: `AwaitHello→Authenticating`, `Authenticating→Pending`,
/// `Authenticating→Active`, `Pending→Active` e qualquer estado → `Closing`.
/// Todas as demais são [`InvalidTransition`].
pub fn transition(from: ConnState, to: ConnState) -> Result<ConnState, InvalidTransition> {
    let valid = matches!(
        (from, to),
        (ConnState::AwaitHello, ConnState::Authenticating)
            | (ConnState::Authenticating, ConnState::Pending)
            | (ConnState::Authenticating, ConnState::Active)
            | (ConnState::Pending, ConnState::Active)
            | (_, ConnState::Closing)
    );
    if valid {
        Ok(to)
    } else {
        Err(InvalidTransition { from, to })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // UT-1.15
    #[test]
    fn state_transitions_table() {
        let all = [
            ConnState::AwaitHello,
            ConnState::Authenticating,
            ConnState::Pending,
            ConnState::Active,
            ConnState::Closing,
        ];
        let valid: [(ConnState, ConnState); 9] = [
            (ConnState::AwaitHello, ConnState::Authenticating),
            (ConnState::Authenticating, ConnState::Pending),
            (ConnState::Authenticating, ConnState::Active),
            (ConnState::Pending, ConnState::Active),
            (ConnState::AwaitHello, ConnState::Closing),
            (ConnState::Authenticating, ConnState::Closing),
            (ConnState::Pending, ConnState::Closing),
            (ConnState::Active, ConnState::Closing),
            (ConnState::Closing, ConnState::Closing),
        ];
        for from in all {
            for to in all {
                let should_pass = valid.iter().any(|(f, t)| *f == from && *t == to);
                match transition(from, to) {
                    Ok(got) => {
                        assert!(should_pass, "{from:?} -> {to:?} deveria ser inválida");
                        assert_eq!(got, to);
                    }
                    Err(InvalidTransition { from: f, to: t }) => {
                        assert!(!should_pass, "{f:?} -> {t:?} deveria ser válida");
                        assert_eq!(f, from);
                        assert_eq!(t, to);
                    }
                }
            }
        }
    }
}
