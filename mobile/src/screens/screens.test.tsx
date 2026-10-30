// F2-T03 — UT-2.11: RTL das telas (Home, Scan, Manual, Aguardando, Transmitindo).

import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { buildPairingUri } from "@localcamera/shared/protocol";
import { renderWithProviders } from "@localcamera/shared/test";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "@/App";
import type { QrScannerApi } from "@/qr/qr-scanner";
import { AppProvider } from "@/state/AppProvider";
import { initialState, type AppState, type Target } from "@/state/connection";

const TARGET: Target = {
  name: "DESKTOP-X",
  host: "192.168.0.10",
  port: 48650,
  rawUri: null,
  token: null,
};

const allGranted = { camera: "granted", mic: "granted", notifications: "granted" } as const;

const homeState: AppState = {
  ...initialState,
  permissions: { ...allGranted },
  permissionsDone: true,
};

function renderApp(state: AppState, scanner?: QrScannerApi) {
  return renderWithProviders(
    <AppProvider initialState={state} qrScanner={scanner}>
      <App />
    </AppProvider>,
  );
}

/** Scanner fake: captura o callback `onCode` para o teste disparar o QR. */
function fakeScanner(): { api: QrScannerApi; onCode: () => ((raw: string) => void) | null } {
  let captured: ((raw: string) => void) | null = null;
  return {
    api: {
      available: true,
      start: (onCode) => {
        captured = onCode;
        return Promise.resolve({
          stop: async () => {},
          setTorch: async () => true,
        });
      },
    },
    onCode: () => captured,
  };
}

// ------------------------------- Home -------------------------------

describe("Home (UT-2.11)", () => {
  it("com PC conhecido mostra Reconectar + 'visto há…'", async () => {
    const state: AppState = {
      ...homeState,
      knownPcs: [
        {
          name: "DESKTOP-X",
          host: "192.168.0.10",
          port: 48650,
          lastSeenMs: Date.now() - 2 * 3600_000,
        },
      ],
    };
    await renderApp(state);
    expect(await screen.findByText("Reconectar a DESKTOP-X")).toBeInTheDocument();
    expect(screen.getByText("visto há 2 horas")).toBeInTheDocument();
    expect(screen.getByText("Meus PCs (1)")).toBeInTheDocument();
  });

  it("sem PC conhecido não mostra Reconectar", async () => {
    await renderApp(homeState);
    expect(await screen.findByText("Pronto para conectar")).toBeInTheDocument();
    expect(screen.queryByText(/Reconectar a/)).not.toBeInTheDocument();
  });

  it("sem rede mostra o cartão de aviso", async () => {
    const original = Object.getOwnPropertyDescriptor(Navigator.prototype, "onLine");
    Object.defineProperty(Navigator.prototype, "onLine", { configurable: true, value: false });
    try {
      await renderApp(homeState);
      expect(
        await screen.findByText("Sem rede. Conecte-se ao Wi-Fi ou use o cabo USB."),
      ).toBeInTheDocument();
    } finally {
      if (original) Object.defineProperty(Navigator.prototype, "onLine", original);
    }
  });

  it("permissão recomendada revogada mostra cartão informativo", async () => {
    const state: AppState = {
      ...homeState,
      permissions: { camera: "granted", mic: "denied", notifications: "granted" },
    };
    await renderApp(state);
    expect(
      await screen.findByText("Permissão negada. Ative nas configurações do Android."),
    ).toBeInTheDocument();
  });
});

// ------------------------------- Scan -------------------------------

describe("Scan (UT-2.11)", () => {
  const scanState: AppState = { ...homeState, route: "scan" };
  const validQr = buildPairingUri({
    v: 1,
    hosts: ["192.168.0.10"],
    port: 48650,
    token: "0123456789abcdef0123456789abcdef",
    pc: "pc1",
    name: "DESKTOP-X",
  });

  it("QR inválido → faixa danger por 3 s e fica na tela", async () => {
    const { api, onCode } = fakeScanner();
    await renderApp(scanState, api);
    await waitFor(() => expect(onCode()).not.toBeNull());

    await act(async () => {
      onCode()!("https://exemplo.com/qr");
    });
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Este QR code não é do LocalCamera.");
    // Ainda na tela Scan (não foi para Conectando).
    expect(screen.getByText("Aponte para o QR code exibido no PC")).toBeInTheDocument();
  });

  it("QR válido → vai para Conectando com o nome do PC", async () => {
    const { api, onCode } = fakeScanner();
    await renderApp(scanState, api);
    await waitFor(() => expect(onCode()).not.toBeNull());

    await act(async () => {
      onCode()!(validQr);
    });
    expect(await screen.findByText("Conectando a DESKTOP-X…")).toBeInTheDocument();
  });
});

// ------------------------------- Manual -------------------------------

describe("Manual (UT-2.11)", () => {
  const manualState: AppState = { ...homeState, route: "manual" };

  it("valida inline ao sair do campo e habilita Conectar só com tudo válido", async () => {
    await renderApp(manualState);
    const user = userEvent.setup();
    const address = screen.getByLabelText("Endereço do PC");
    const port = screen.getByLabelText("Porta");
    const code = screen.getByLabelText("Código de pareamento");

    expect(screen.getByRole("button", { name: "Conectar" })).toBeDisabled();

    await user.type(address, "a b"); // espaço: não é IP nem hostname
    await user.tab();
    expect(screen.getByText("Endereço inválido.")).toBeInTheDocument();

    await user.clear(port);
    await user.type(port, "70000");
    await user.tab();
    expect(screen.getByText("Porta inválida.")).toBeInTheDocument();

    await user.type(code, "xyz");
    await user.tab();
    expect(screen.getByText("O código tem 32 caracteres (0–9, a–f).")).toBeInTheDocument();

    await user.clear(address);
    await user.type(address, "192.168.0.10");
    await user.clear(port);
    await user.type(port, "48650");
    await user.clear(code);
    await user.type(code, "0123456789abcdef0123456789abcdef");

    const connect = screen.getByRole("button", { name: "Conectar" });
    expect(connect).toBeEnabled();
    await user.click(connect);
    expect(await screen.findByText("Conectando a 192.168.0.10…")).toBeInTheDocument();
  });
});

// ------------------------------- Aguardando -------------------------------

describe("Aguardando aprovação (UT-2.11)", () => {
  beforeEach(() => {
    // Só `setInterval` fica fake: o i18next usa `setTimeout` no init.
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("conta regressivo e vira erro not_approved ao zerar", async () => {
    const state: AppState = {
      ...homeState,
      connection: { kind: "pending", target: TARGET, timeoutS: 2 },
    };
    await renderApp(state);

    expect(screen.getByText("Aprove no PC")).toBeInTheDocument();
    const countdown = screen.getByRole("progressbar");
    expect(countdown).toHaveAttribute("aria-valuenow", "2");

    await act(async () => {
      vi.advanceTimersByTime(1000);
    });
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "1");

    await act(async () => {
      vi.advanceTimersByTime(1000);
    });
    // Zerou sem resposta → erro definitivo not_approved.
    expect(await screen.findByText("Conexão não aprovada")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Tentar de novo" })).toBeInTheDocument();
  });
});

// ------------------------------- Transmitindo -------------------------------

describe("Transmitindo (UT-2.11)", () => {
  const streamingState: AppState = {
    ...homeState,
    connection: { kind: "streaming", target: TARGET },
    stream: {
      ...initialState.stream,
      stats: { fps: 30, bitrateKbps: 8000, rttMs: 12, dropped: 0 },
      warnings: { slow: true, hot: false, battery: false },
    },
  };

  it("mostra tiles, aviso de rede lenta e Dupla desabilitada", async () => {
    await renderApp(streamingState);

    expect(await screen.findByText("Transmitindo")).toBeInTheDocument();
    expect(screen.getByText("DESKTOP-X")).toBeInTheDocument();

    // StatTiles 2×2
    expect(screen.getByText("8.0")).toBeInTheDocument();
    expect(screen.getByText("Mbps")).toBeInTheDocument();
    expect(screen.getByText("12")).toBeInTheDocument();
    expect(screen.getByText("ms")).toBeInTheDocument();
    expect(screen.getByText("0")).toBeInTheDocument();
    // "Traseira" aparece no tile Câmera e no rádio selecionado do controle —
    // ambos derivados do mesmo estado (stream.camera = "back").
    expect(screen.getAllByText("Traseira")).toHaveLength(2);

    // Aviso de rede lenta
    expect(screen.getByText("Rede lenta — qualidade reduzida.")).toBeInTheDocument();

    // Dupla indisponível (caps.concurrent = false) com motivo
    const dual = screen.getByRole("radio", { name: "Dupla" });
    expect(dual).toBeDisabled();
    expect(dual).toHaveAttribute("title", "Câmera dupla indisponível neste aparelho.");

    // Parar → Home
    await userEvent.setup().click(screen.getByRole("button", { name: "Parar" }));
    expect(await screen.findByText("Pronto para conectar")).toBeInTheDocument();
  });
});
