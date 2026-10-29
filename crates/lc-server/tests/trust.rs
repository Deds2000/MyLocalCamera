//! Testes de integração do módulo `trust` (F1-T05).

use std::fs;
use std::path::Path;

use lc_server::trust::{secrets, TrustStore, VerifyDevice};

/// Tempo fixo (segundos Unix) para determinismo dos testes.
const NOW: u64 = 1_700_000_000;

fn open(dir: &Path) -> TrustStore {
    TrustStore::open(dir).unwrap()
}

// Módulo `trust` para o filtro `cargo test -p lc-server trust` casar com
// os nomes dos testes (`trust::<nome>`).
mod trust {
    use super::*;

    /// UT-1.13 — o `trust.json` guarda o hash, nunca a chave em claro.
    #[test]
    fn only_hash_is_persisted() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = open(tmp.path());
        let key = store
            .add_device("dev-1", "Fone", "192.168.0.10", NOW)
            .unwrap();

        let text = fs::read_to_string(tmp.path().join("trust.json")).unwrap();
        assert!(!text.contains(&key), "deviceKey em claro foi gravada");
        assert!(
            text.contains(&secrets::sha256_hex(&key)),
            "hash SHA-256 da chave deve estar no arquivo"
        );
    }

    /// UT-1.17 — `regenerate_token` invalida o token antigo.
    #[test]
    fn token_rotates_and_old_is_rejected() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = open(tmp.path());

        let old = store.pair_token().to_string();
        assert_eq!(old.len(), 32);

        let new = store.regenerate_token().unwrap();
        assert_eq!(new.len(), 32);
        assert_ne!(new, old);

        assert!(
            !store.verify_pair_token(&old),
            "token antigo deve ser recusado"
        );
        assert!(store.verify_pair_token(&new), "token novo deve ser aceito");
    }

    /// UT-1.25 — JSON truncado, arquivo vazio e `schemaVersion: 99`:
    /// backup `.corrupt-*` é criado e o store novo fica funcional.
    #[test]
    fn corrupt_file_is_backed_up_and_recreated() {
        let cases = [
        ("truncado", "{\"schemaVersion\":1,\"pcId\":\"abc"),
        ("vazio", ""),
        (
            "schema-futuro",
            "{\"schemaVersion\": 99, \"pcId\": \"abc\", \"pairToken\": \"00\", \"devices\": []}",
        ),
    ];
        for (label, content) in cases {
            let tmp = tempfile::TempDir::new().unwrap();
            fs::write(tmp.path().join("trust.json"), content).unwrap();

            let mut store = open(tmp.path());

            let backups = fs::read_dir(tmp.path())
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|n| n.starts_with("trust.json.corrupt-"))
                .count();
            assert_eq!(backups, 1, "caso {label}: backup .corrupt-* esperado");

            // store novo é funcional
            let key = store
                .add_device("dev-1", "Fone", "192.168.0.10", NOW)
                .unwrap();
            assert_eq!(store.verify_device("dev-1", &key), VerifyDevice::Ok);
            assert_eq!(store.list().len(), 1);
        }
    }

    /// IT — o store persiste entre aberturas (mesmo `pcId` = AT-1.15).
    #[test]
    fn persists_across_reopen() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (key, pc_id) = {
            let mut store = open(tmp.path());
            let key = store
                .add_device("dev-1", "Fone", "192.168.0.10", NOW)
                .unwrap();
            (key, store.pc_id().to_string())
        };

        let store = open(tmp.path());
        assert_eq!(store.pc_id(), pc_id, "pcId deve ser estável");
        assert_eq!(store.verify_device("dev-1", &key), VerifyDevice::Ok);
    }

    /// IT — revogar faz `verify_device` retornar `Unknown`.
    #[test]
    fn revoke_makes_device_unknown() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = open(tmp.path());
        let key = store
            .add_device("dev-1", "Fone", "192.168.0.10", NOW)
            .unwrap();
        assert_eq!(store.verify_device("dev-1", &key), VerifyDevice::Ok);

        assert!(store.revoke("dev-1").unwrap());
        assert_eq!(store.verify_device("dev-1", &key), VerifyDevice::Unknown);
        assert!(!store.revoke("dev-1").unwrap(), "revogar de novo → false");
    }

    /// IT — chave errada (cadastrado, mas hash não confere) → `BadKey`.
    #[test]
    fn bad_key_detected() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = open(tmp.path());
        let key = store
            .add_device("dev-1", "Fone", "192.168.0.10", NOW)
            .unwrap();

        let wrong = {
            let mut v = key.clone().into_bytes();
            v[0] = if v[0] == b'0' { b'1' } else { b'0' };
            String::from_utf8(v).unwrap()
        };

        assert_eq!(store.verify_device("dev-1", &wrong), VerifyDevice::BadKey);
    }

    /// ST-1.01 — `Debug` da `TrustStore` não vaza token nem hash.
    #[test]
    fn debug_does_not_leak() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = open(tmp.path());
        let key = store
            .add_device("dev-1", "Fone", "192.168.0.10", NOW)
            .unwrap();
        let token = store.pair_token().to_string();
        let hash = secrets::sha256_hex(&key);

        let dbg = format!("{store:?}");
        assert!(!dbg.contains(&token), "Debug vazou o token de pareamento");
        assert!(!dbg.contains(&hash), "Debug vazou o hash da deviceKey");
    }
}
