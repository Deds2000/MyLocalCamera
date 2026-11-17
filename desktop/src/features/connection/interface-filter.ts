import { buildPairingUri, parsePairingUri } from "@localcamera/shared/protocol";
import type { ConnectionInfo, NetworkInterface } from "@/lib/tauri";

export interface FilteredInterfaces {
  shown: NetworkInterface[];
  defaultIp: string | null;
}

export interface PairingLink {
  uri: string;
  hosts: string[];
  ip: string;
}

const OBVIOUS_VIRTUAL_ADAPTER =
  /hyper[\s-]?v|vmware|virtualbox|vpn|tunnel|\btun\d*\b|\btap\d*\b|wsl|docker|loopback|tailscale|wireguard|zerotier/i;

function isVirtual(networkInterface: NetworkInterface): boolean {
  return networkInterface.isVirtual || OBVIOUS_VIRTUAL_ADAPTER.test(networkInterface.name);
}

/** Keep detected physical interfaces visible; include a virtual one only when explicitly chosen. */
export function filterInterfaces(
  interfaces: NetworkInterface[],
  userChoice?: string | null,
): FilteredInterfaces {
  const real = interfaces.filter((networkInterface) => !isVirtual(networkInterface));
  const chosen = userChoice
    ? interfaces.find((networkInterface) => networkInterface.ip === userChoice)
    : undefined;
  const shown = [...real];
  if (chosen && isVirtual(chosen)) shown.push(chosen);
  return {
    shown,
    defaultIp: chosen?.ip ?? real[0]?.ip ?? null,
  };
}

/** Rebuild only from server-provided data; always use its effective port. */
export function pairingLinkForInterface(
  info: ConnectionInfo,
  userChoice?: string | null,
): PairingLink | null {
  const parsed = parsePairingUri(info.qrcodeUri);
  if (!parsed.ok) return null;

  const filtered = filterInterfaces(info.interfaces, userChoice);
  const selected =
    userChoice && filtered.shown.some((item) => item.ip === userChoice) ? userChoice : null;
  const visibleIps = new Set(filtered.shown.map((item) => item.ip));
  const hosts = selected
    ? [selected]
    : info.interfaces.length === 0
      ? parsed.value.hosts
      : parsed.value.hosts.filter((ip) => visibleIps.has(ip));
  if (hosts.length === 0) return null;
  const ip = selected ?? filtered.defaultIp ?? hosts[0];
  if (!ip) return null;

  return {
    uri: buildPairingUri({ ...parsed.value, hosts, port: info.effectivePort }),
    hosts,
    ip,
  };
}
