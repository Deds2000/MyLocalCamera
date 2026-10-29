//! Parser da URI de pareamento (`docs/pairing-uri-vectors.json`).
//!
//! As regras são as mesmas do parser TypeScript (`packages/shared`) e
//! serão reimplementadas em Kotlin na Fase 2: `v` inteiro (`v > 1` =
//! incompatível), 1–4 hosts IPv4, porta 1–65535, token 32 hex, `pc` não
//! vazio (≤ 64) e `name` ≤ 64 (decodificado). Parâmetros desconhecidos
//! são ignorados.

use std::net::Ipv4Addr;

use url::Url;

/// Campos da URI de pareamento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingUri {
    pub v: u32,
    pub hosts: Vec<Ipv4Addr>,
    pub port: u16,
    pub token: String,
    pub pc: String,
    pub name: String,
}

/// Falha do parser (mesma taxonomia dos vetores).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UriError {
    /// URI malformada ou campo fora das regras.
    #[error("URI de pareamento inválida")]
    Invalid,
    /// `v > 1`: gerado por uma versão mais nova do app.
    #[error("versão de protocolo incompatível (v>1)")]
    IncompatibleVersion,
}

fn only_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn is_hex32(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Faz o parse da URI `localcamera://connect?...`.
///
/// # Errors
/// [`UriError::Invalid`] para qualquer campo inválido;
/// [`UriError::IncompatibleVersion`] quando `v > 1`.
pub fn parse_pairing_uri(input: &str) -> Result<PairingUri, UriError> {
    let url = Url::parse(input).map_err(|_| UriError::Invalid)?;
    if url.scheme() != "localcamera" || url.host_str() != Some("connect") {
        return Err(UriError::Invalid);
    }
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    // `URLSearchParams.get` devolve o primeiro valor; idem aqui.
    let param = |key: &str| {
        pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };

    let v_raw = param("v").ok_or(UriError::Invalid)?;
    if !only_digits(v_raw) {
        return Err(UriError::Invalid);
    }
    let v: u32 = v_raw.parse().map_err(|_| UriError::Invalid)?;
    if v > 1 {
        return Err(UriError::IncompatibleVersion);
    }

    let hosts_raw = param("hosts").ok_or(UriError::Invalid)?;
    if hosts_raw.is_empty() {
        return Err(UriError::Invalid);
    }
    let host_parts: Vec<&str> = hosts_raw.split(',').collect();
    if host_parts.is_empty() || host_parts.len() > 4 {
        return Err(UriError::Invalid);
    }
    let mut hosts = Vec::with_capacity(host_parts.len());
    for part in host_parts {
        hosts.push(part.parse::<Ipv4Addr>().map_err(|_| UriError::Invalid)?);
    }

    let port_raw = param("port").ok_or(UriError::Invalid)?;
    if !only_digits(port_raw) {
        return Err(UriError::Invalid);
    }
    let port: u32 = port_raw.parse().map_err(|_| UriError::Invalid)?;
    if port == 0 || port > u32::from(u16::MAX) {
        return Err(UriError::Invalid);
    }

    let token = param("token").ok_or(UriError::Invalid)?;
    if !is_hex32(token) {
        return Err(UriError::Invalid);
    }

    let pc = param("pc").ok_or(UriError::Invalid)?;
    if pc.is_empty() || pc.chars().count() > 64 {
        return Err(UriError::Invalid);
    }

    let name = param("name").ok_or(UriError::Invalid)?;
    if name.chars().count() > 64 {
        return Err(UriError::Invalid);
    }

    Ok(PairingUri {
        v,
        hosts,
        port: u16::try_from(port).expect("porta validada até 65535"),
        token: token.to_owned(),
        pc: pc.to_owned(),
        name: name.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_pairing_uri, UriError};
    use serde_json::Value;

    /// UT `uri_vectors`: compara com `docs/pairing-uri-vectors.json`.
    #[test]
    fn uri_vectors() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/pairing-uri-vectors.json"
        );
        let raw = std::fs::read_to_string(path).expect("ler docs/pairing-uri-vectors.json");
        let vectors: Vec<Value> = serde_json::from_str(&raw).expect("JSON dos vetores");
        for vector in &vectors {
            let name = vector["name"].as_str().expect("nome");
            let uri = vector["uri"].as_str().expect("uri");
            let expect = vector["expect"].as_str().expect("expect");
            let result = parse_pairing_uri(uri);
            match expect {
                "ok" => {
                    let got = result.unwrap_or_else(|e| panic!("{name}: {e}"));
                    let value = &vector["value"];
                    assert_eq!(
                        u64::from(got.v),
                        value["v"].as_u64().expect("v"),
                        "{name}: v"
                    );
                    let hosts: Vec<String> = value["hosts"]
                        .as_array()
                        .expect("hosts")
                        .iter()
                        .map(|h| h.as_str().expect("host").to_owned())
                        .collect();
                    assert_eq!(
                        got.hosts
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>(),
                        hosts,
                        "{name}: hosts"
                    );
                    assert_eq!(
                        u64::from(got.port),
                        value["port"].as_u64().expect("port"),
                        "{name}: port"
                    );
                    assert_eq!(
                        got.token,
                        value["token"].as_str().expect("token"),
                        "{name}: token"
                    );
                    assert_eq!(got.pc, value["pc"].as_str().expect("pc"), "{name}: pc");
                    assert_eq!(
                        got.name,
                        value["name"].as_str().expect("name"),
                        "{name}: name"
                    );
                }
                "invalid" => assert_eq!(result, Err(UriError::Invalid), "{name}"),
                "incompatible_version" => {
                    assert_eq!(result, Err(UriError::IncompatibleVersion), "{name}");
                }
                other => panic!("expect desconhecido: {other}"),
            }
        }
    }
}
