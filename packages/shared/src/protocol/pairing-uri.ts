/** URI de pareamento: `localcamera://connect?v=1&hosts=IP1,IP2&port=48650&token=<32 hex>&pc=<id>&name=<nome>`.
 *
 * O mesmo parser será reimplementado em Kotlin na Fase 2 (F2-T05a) com estes
 * vetores (`docs/pairing-uri-vectors.json`).
 */

export type PairingUri = {
  v: number;
  hosts: string[];
  port: number;
  token: string;
  pc: string;
  name: string;
};

export type ParseResult =
  | { ok: true; value: PairingUri }
  | { ok: false; error: "invalid" | "incompatible_version" };

const IP_OCTET = "(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])";
const IPV4 = new RegExp(`^${IP_OCTET}(\\.${IP_OCTET}){3}$`);
const TOKEN = /^[0-9a-fA-F]{32}$/;
const UNSIGNED_INT = /^[0-9]+$/;

/** Monta a URI a partir dos campos (ordem fixa: v, hosts, port, token, pc, name). */
export function buildPairingUri(p: PairingUri): string {
  const params = new URLSearchParams();
  params.set("v", String(p.v));
  params.set("hosts", p.hosts.join(","));
  params.set("port", String(p.port));
  params.set("token", p.token);
  params.set("pc", p.pc);
  params.set("name", p.name);
  return `localcamera://connect?${params.toString()}`;
}

/** Regras: v inteiro (v > 1 = incompatível); 1–4 hosts IPv4; port 1–65535;
 * token = 32 hex; pc não vazio (≤ 64); name ≤ 64 (decodificado);
 * parâmetros desconhecidos são ignorados. */
export function parsePairingUri(uri: string): ParseResult {
  let url: URL;
  try {
    url = new URL(uri);
  } catch {
    return { ok: false, error: "invalid" };
  }
  if (url.protocol !== "localcamera:") return { ok: false, error: "invalid" };
  if (url.host !== "connect") return { ok: false, error: "invalid" };

  const q = url.searchParams;

  const vRaw = q.get("v");
  if (vRaw === null || !UNSIGNED_INT.test(vRaw)) return { ok: false, error: "invalid" };
  const v = Number(vRaw);
  if (v > 1) return { ok: false, error: "incompatible_version" };

  const hostsRaw = q.get("hosts");
  if (hostsRaw === null || hostsRaw === "") return { ok: false, error: "invalid" };
  const hosts = hostsRaw.split(",");
  if (hosts.length > 4) return { ok: false, error: "invalid" };
  if (hosts.some((h) => !IPV4.test(h))) return { ok: false, error: "invalid" };

  const portRaw = q.get("port");
  if (portRaw === null || !UNSIGNED_INT.test(portRaw)) return { ok: false, error: "invalid" };
  const port = Number(portRaw);
  if (port < 1 || port > 65535) return { ok: false, error: "invalid" };

  const token = q.get("token");
  if (token === null || !TOKEN.test(token)) return { ok: false, error: "invalid" };

  const pc = q.get("pc");
  if (pc === null || pc.length === 0 || pc.length > 64) return { ok: false, error: "invalid" };

  const name = q.get("name");
  if (name === null || name.length > 64) return { ok: false, error: "invalid" };

  return { ok: true, value: { v, hosts, port, token, pc, name } };
}
