//! Geração e comparação de segredos.
//!
//! - Aleatoriedade vem sempre do **sistema operacional** (`getrandom`).
//! - A `deviceKey` em claro existe só na memória e só no momento do
//!   pareamento: nunca é gravada, logada nem serializada.

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Gera `bytes` bytes aleatórios do RNG do sistema e devolve em hex
/// minúsculo (2 caracteres por byte).
///
/// # Pânico
///
/// Pânico se o sistema negar fornecer entropia (segurança primeiro:
/// seguir com zeros seria pior).
pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).expect("falha no RNG do sistema");
    hex::encode(buf)
}

/// SHA-256 de `input` em hex minúsculo (64 caracteres).
#[must_use]
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// Compara dois segredos em **tempo constante**.
///
/// Retorna `false` se os tamanhos diferem (sem panic, sem comparação
/// parcial). Usar para token de pareamento e hashes de `deviceKey`.
#[must_use]
pub fn ct_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// UT-1.12 — 10 000 tokens (16 B) e 10 000 chaves (32 B):
    /// shape correto, só `[0-9a-f]`, sem colisões.
    #[test]
    fn generated_secrets_shape_and_uniqueness() {
        let mut tokens = HashSet::with_capacity(10_000);
        for _ in 0..10_000 {
            let t = random_hex(16);
            assert_eq!(t.len(), 32, "token com tamanho errado");
            assert!(t.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
            assert!(tokens.insert(t), "token duplicado");
        }

        let mut keys = HashSet::with_capacity(10_000);
        for _ in 0..10_000 {
            let k = random_hex(32);
            assert_eq!(k.len(), 64, "chave com tamanho errado");
            assert!(k.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
            assert!(keys.insert(k), "chave duplicada");
        }
    }

    /// UT-1.14 — tamanhos 0, 1, 31, 32, 64 e 1000: `false` sem panic;
    /// strings iguais dão `true`.
    #[test]
    fn ct_eq_different_lengths_no_panic() {
        for n in [0usize, 1, 31, 32, 64, 1000] {
            let a = "a".repeat(n);
            let m = n + 1;
            let c = "a".repeat(m);
            assert!(
                !ct_eq(&a, &c),
                "tamanhos diferentes devem dar false: {n} vs {m}"
            );
            if n > 0 {
                let b = "b".repeat(n);
                assert!(!ct_eq(&a, &b), "mesmo tamanho, conteudo diferente: n={n}");
            }
        }
        assert!(ct_eq("", ""), "strings iguais (vazias) → true");
        assert!(ct_eq("0123456789abcdef", "0123456789abcdef"));
        assert!(!ct_eq("0123456789abcdef", "0123456789abcde0"));
    }
}
