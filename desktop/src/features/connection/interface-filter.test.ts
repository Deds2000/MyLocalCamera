import { buildPairingUri, parsePairingUri, type PairingUri } from "@localcamera/shared/protocol";
import type { ConnectionInfo } from "@/lib/tauri";
import pairingUriVectors from "../../../../docs/pairing-uri-vectors.json?raw";
import { describe, expect, it } from "vitest";
import { filterInterfaces, pairingLinkForInterface } from "./interface-filter";

describe("filtro de interfaces e URI de pareamento", () => {
  it("UT-3.12 mantém interfaces reais primeiro e inclui a virtual escolhida", () => {
    const interfaces = [
      { name: "Hyper-V Ethernet", ip: "172.22.0.1", isVirtual: false },
      { name: "Wi-Fi", ip: "192.168.1.20", isVirtual: false },
      { name: "VPN tunnel", ip: "10.8.0.2", isVirtual: false },
      { name: "VMware Network Adapter", ip: "192.168.56.1", isVirtual: true },
    ];

    const defaults = filterInterfaces(interfaces);
    expect(defaults.shown.map((item) => item.name)).toEqual(["Wi-Fi"]);
    expect(defaults.defaultIp).toBe("192.168.1.20");

    const selected = filterInterfaces(interfaces, "10.8.0.2");
    expect(selected.shown.map((item) => item.name)).toEqual(["Wi-Fi", "VPN tunnel"]);
    expect(selected.defaultIp).toBe("10.8.0.2");
  });

  it("UT-3.13 gera URI parseável com hosts, porta efetiva e caracteres reservados", () => {
    const vectors = JSON.parse(pairingUriVectors) as Array<{ expect: string; uri: string }>;
    const validVectors = vectors.filter((vector) => vector.expect === "ok");
    expect(validVectors.length).toBeGreaterThan(0);
    validVectors.forEach((vector) => expect(parsePairingUri(vector.uri).ok).toBe(true));

    const cases: Array<{ hosts: string[]; port: number; name: string }> = [
      { hosts: ["192.168.1.20"], port: 48650, name: "Câmera da sala & cozinha" },
      { hosts: ["192.168.1.20", "10.0.0.7"], port: 50123, name: "PC çá & vídeo" },
    ];
    for (const item of cases) {
      const info = connectionInfo(item);
      const link = pairingLinkForInterface(info);
      expect(link).not.toBeNull();
      const parsed = parsePairingUri(link!.uri);
      expect(parsed.ok).toBe(true);
      if (parsed.ok) {
        expect(parsed.value.hosts).toEqual(item.hosts);
        expect(parsed.value.port).toBe(item.port);
        expect(parsed.value.token).toBe("0123456789abcdef0123456789abcdef");
        expect(parsed.value.pc).toBe("local-camera-pc");
        expect(parsed.value.name).toBe(item.name);
      }
    }
  });

  it("a escolha de uma interface limita hosts e mantém a porta efetiva", () => {
    const info = connectionInfo({
      hosts: ["192.168.1.20", "10.0.0.7"],
      port: 48650,
      name: "LocalCamera",
    });
    const link = pairingLinkForInterface(info, "10.0.0.7");
    expect(link?.hosts).toEqual(["10.0.0.7"]);
    expect(parsePairingUri(link!.uri)).toMatchObject({
      ok: true,
      value: { hosts: ["10.0.0.7"], port: info.effectivePort },
    });
  });
});

function connectionInfo(pairing: Pick<PairingUri, "hosts" | "port" | "name">): ConnectionInfo {
  const value: PairingUri = {
    v: 1,
    ...pairing,
    token: "0123456789abcdef0123456789abcdef",
    pc: "local-camera-pc",
  };
  return {
    port: pairing.port === 50123 ? 48650 : pairing.port,
    effectivePort: pairing.port,
    interfaces: pairing.hosts.map((ip, index) => ({
      name: index === 0 ? "Ethernet" : "Wi-Fi",
      ip,
      isVirtual: false,
    })),
    qrcodeUri: buildPairingUri(value),
    pairCodeMasked: "0123••••",
    pcId: value.pc,
    name: value.name,
  };
}
