//! Redação de segredos nos logs (F1-T16).
//!
//! Nenhum segredo (token de pareamento, `deviceKey`, `audioTicket`) pode
//! aparecer inteiro em log. Quando for necessário identificar um valor,
//! use [`Redacted`] (ou [`short`]) — no máximo 4 caracteres.

use std::fmt;

/// Mostra no máximo os 4 primeiros caracteres de um segredo, seguidos de
/// `…` (reticências). Strings com menos de 4 caracteres viram só `…`.
///
/// ```
/// # use lc_server::redact::short;
/// assert_eq!(short("abcdef"), "abcd…");
/// assert_eq!(short("ab"), "…");
/// assert_eq!(short(""), "…");
/// ```
#[must_use]
pub fn short(secret: &str) -> String {
    let prefix: String = secret.chars().take(4).collect();
    if prefix.chars().count() < 4 {
        "…".to_owned()
    } else {
        format!("{prefix}…")
    }
}

/// Envolve um segredo para uso direto em logs (`Display` usa [`short`]).
///
/// ```
/// # use lc_server::redact::Redacted;
/// assert_eq!(format!("{}", Redacted("abcdef")), "abcd…");
/// ```
pub struct Redacted<'a>(pub &'a str);

impl fmt::Display for Redacted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&short(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::{short, Redacted};

    /// UT: primeiros 4 caracteres + `…`; menos de 4 → só `…`.
    #[test]
    fn short_formats() {
        assert_eq!(short("abcdef"), "abcd…");
        assert_eq!(short("abcd"), "abcd…");
        assert_eq!(short("ab"), "…");
        assert_eq!(short(""), "…");
    }

    /// UT: `Display` de `Redacted` usa `short`.
    #[test]
    fn redacted_display() {
        assert_eq!(format!("{}", Redacted("abcdef")), "abcd…");
        assert_eq!(format!("{}", Redacted("ab")), "…");
    }
}
