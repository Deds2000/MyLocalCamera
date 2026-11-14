use std::net::IpAddr;

use if_addrs::{Interface, get_if_addrs};
use serde::Serialize;

use super::CommandError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InterfaceInfo {
    name: String,
    ip: String,
    is_virtual: bool,
}

pub(super) fn list_interfaces() -> Result<Vec<InterfaceInfo>, CommandError> {
    let interfaces = get_if_addrs().map_err(|_| CommandError::InterfaceEnumerationFailed)?;
    Ok(interfaces
        .into_iter()
        .filter(Interface::is_oper_up)
        .filter_map(|interface| {
            let ip = interface.ip();
            if !ip.is_ipv4() {
                return None;
            }
            Some(InterfaceInfo {
                name: interface.name.clone(),
                ip: ip.to_string(),
                is_virtual: is_virtual_interface(&interface),
            })
        })
        .collect())
}

pub(super) fn pairing_hosts(interfaces: &[InterfaceInfo]) -> Vec<String> {
    interfaces
        .iter()
        .filter_map(|interface| {
            let ip = interface.ip.parse::<IpAddr>().ok()?;
            (!ip.is_loopback() && !is_ipv4_link_local(ip)).then(|| ip.to_string())
        })
        .collect()
}

fn is_ipv4_link_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            octets[0] == 169 && octets[1] == 254
        }
        IpAddr::V6(_) => false,
    }
}

fn is_virtual_interface(interface: &Interface) -> bool {
    const VIRTUAL_MARKERS: &[&str] = &[
        "virtual",
        "vmware",
        "vbox",
        "virtualbox",
        "hyper-v",
        "wsl",
        "docker",
        "tap",
        "tun",
        "tailscale",
        "zerotier",
        "wireguard",
        "vethernet",
        "npcap",
        "loopback",
    ];
    let name = interface.name.to_ascii_lowercase();
    interface.is_p2p() || VIRTUAL_MARKERS.iter().any(|marker| name.contains(marker))
}

pub(super) fn pairing_uri(
    hosts: &[String],
    port: u16,
    token: &str,
    pc_id: &str,
    name: &str,
) -> String {
    format!(
        "localcamera://connect?v=1&hosts={}&port={port}&token={token}&pc={pc_id}&name={}",
        hosts.join(","),
        percent_encode(name),
    )
}

pub(super) fn mask_pair_token(token: &str) -> String {
    format!("{}••••", token.chars().take(4).collect::<String>())
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::percent_encode;

    #[test]
    fn percent_encodes_utf8_and_reserved_query_characters() {
        assert_eq!(
            percent_encode("My PC & câmera"),
            "My%20PC%20%26%20c%C3%A2mera"
        );
    }
}
