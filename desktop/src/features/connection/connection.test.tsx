import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "@localcamera/shared/test";
import { buildPairingUri } from "@localcamera/shared/protocol";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ConnectionInfo, TauriAdapter } from "@/lib/tauri";
import { createTauriClient } from "@/lib/tauri";
import { createAppStore } from "@/store";
import { ConnectionTab } from "./ConnectionTab";

describe("ConnectionTab (F3-T04)", () => {
  beforeEach(() => {
    localStorage.clear();
    Object.defineProperty(HTMLElement.prototype, "hasPointerCapture", {
      configurable: true,
      value: () => false,
    });
    Object.defineProperty(HTMLElement.prototype, "setPointerCapture", {
      configurable: true,
      value: () => undefined,
    });
    Object.defineProperty(HTMLElement.prototype, "releasePointerCapture", {
      configurable: true,
      value: () => undefined,
    });
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
      configurable: true,
      value: () => undefined,
    });
  });

  it("UT-3.23 só regenera o QR após confirmar; cancelar não altera nada", async () => {
    const user = userEvent.setup();
    const fixture = pairingClient();
    const store = createAppStore(fixture.client);
    const { container } = await renderWithProviders(<ConnectionTab store={store} />);
    const qr = () => container.querySelector('[data-testid="qr-card"] svg')?.outerHTML;

    await act(async () => { await waitFor(() => expect(qr()).toBeTruthy()); });
    await act(async () => { await Promise.resolve(); });
    const initialQr = qr();
    expect(screen.getByTestId("pairing-code")).toHaveTextContent("0123••••");

    await user.click(screen.getByTestId("regenerate-qr"));
    expect(await screen.findByTestId("regenerate-dialog")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancelar" }));
    await waitFor(() => expect(screen.queryByTestId("regenerate-dialog")).not.toBeInTheDocument());
    expect(fixture.regenerateCount()).toBe(0);
    expect(qr()).toBe(initialQr);

    await user.click(screen.getByTestId("regenerate-qr"));
    await user.click(await screen.findByRole("button", { name: "Confirmar" }));
    await waitFor(() => expect(qr()).not.toBe(initialQr));
    expect(fixture.regenerateCount()).toBe(1);
  });

  it("selecionar interface atualiza QR, usa porta efetiva e persiste a escolha", async () => {
    const user = userEvent.setup();
    const fixture = pairingClient({ effectivePort: 50123 });
    const store = createAppStore(fixture.client);
    const { container } = await renderWithProviders(<ConnectionTab store={store} />);
    const qr = () => container.querySelector('[data-testid="qr-card"] svg')?.outerHTML;

    await act(async () => { await waitFor(() => expect(qr()).toBeTruthy()); });
    await act(async () => { await Promise.resolve(); });
    const allNetworksQr = qr();
    await user.click(screen.getByTestId("interface-select"));
    await user.click(await screen.findByRole("option", { name: "Wi-Fi · 10.0.0.7" }));

    await act(async () => { await waitFor(() => expect(qr()).not.toBe(allNetworksQr)); });
    expect(screen.getByTestId("network-caption")).toHaveTextContent("10.0.0.7");
    expect(screen.getByTestId("network-caption")).toHaveTextContent("50123");
    expect(store.getState().settings.networkInterface).toBe("10.0.0.7");
  });

  it("mantém o código mascarado até a ação e copia apenas após pedido explícito", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    const fixture = pairingClient();
    const store = createAppStore(fixture.client);
    await renderWithProviders(<ConnectionTab store={store} />);

    await screen.findByTestId("pairing-code");
    expect(screen.getByTestId("pairing-code")).toHaveTextContent("0123••••");
    await user.click(screen.getByRole("button", { name: "Mostrar código de pareamento" }));
    expect(screen.getByTestId("pairing-code")).toHaveTextContent("0123456789abcdef0123456789abcdef");
    await user.click(screen.getByRole("button", { name: "Copiar" }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith("0123456789abcdef0123456789abcdef"));
  });
});

function pairingClient(options: { effectivePort?: number } = {}) {
  let token = "0123456789abcdef0123456789abcdef";
  let regenerateCalls = 0;
  const getInfo = (): ConnectionInfo => ({
    port: 48650,
    effectivePort: options.effectivePort ?? 48650,
    interfaces: [
      { name: "Ethernet", ip: "192.168.1.20", isVirtual: false },
      { name: "Wi-Fi", ip: "10.0.0.7", isVirtual: false },
      { name: "Hyper-V", ip: "172.22.0.1", isVirtual: true },
    ],
    qrcodeUri: buildPairingUri({
      v: 1,
      hosts: ["192.168.1.20", "10.0.0.7"],
      port: options.effectivePort ?? 48650,
      token,
      pc: "localcamera-pc",
      name: "LocalCamera PC",
    }),
    pairCodeMasked: `${token.slice(0, 4)}••••`,
    pcId: "localcamera-pc",
    name: "LocalCamera PC",
  });
  const adapter: TauriAdapter = {
    async invoke<T>(command: string): Promise<T> {
      switch (command) {
        case "get_connection_info":
          return getInfo() as T;
        case "regenerate_token":
          token = "fedcba9876543210fedcba9876543210";
          regenerateCalls += 1;
          return token as T;
        case "list_trusted":
          return [] as T;
        case "get_health":
          return {
            server: { state: "listening", port: 48650, effectivePort: 48650 },
            stubs: [],
          } as T;
        default:
          throw new Error("unsupported test command");
      }
    },
    async listen() {
      return () => undefined;
    },
  };
  return {
    client: createTauriClient(adapter),
    regenerateCount: () => regenerateCalls,
  };
}
