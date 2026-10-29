//! Estado persistido do mock: `{deviceId, deviceKey}` (texto simples —
//! é só uma ferramenta de teste; o formato é o mesmo do cartão F1-T11a).

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Dados salvos após um pareamento aprovado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub device_id: String,
    pub device_key: String,
}

/// Carrega o estado; arquivo ausente ou inválido → `None`.
#[must_use]
pub fn load(path: &Path) -> Option<State> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Salva o estado no caminho indicado (sobrescreve).
///
/// # Errors
/// Erros de I/O ou de serialização do JSON.
pub fn save(path: &Path, state: &State) -> std::io::Result<()> {
    let raw = serde_json::to_string_pretty(state).map_err(std::io::Error::other)?;
    std::fs::write(path, raw)
}

#[cfg(test)]
mod tests {
    use super::{load, save, State};

    /// UT `state_file_roundtrip`: salvar e carregar devolve o mesmo estado.
    #[test]
    fn state_file_roundtrip() {
        let path =
            std::env::temp_dir().join(format!("mock-sender-state-{}.json", uuid::Uuid::new_v4()));
        let state = State {
            device_id: "dev-1".to_owned(),
            device_key: "ab".repeat(32),
        };
        save(&path, &state).expect("save");
        assert_eq!(load(&path), Some(state));
        let _ = std::fs::remove_file(&path);
        assert_eq!(load(&path), None);
    }
}
