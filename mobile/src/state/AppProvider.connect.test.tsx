// F2-T13 (correção 2026-11-03): o AppProvider precisa chamar o plugin
// nativo ao entrar em `connecting` (antes só o reducer local era atualizado)
// e encerrar o serviço quando a UI volta para idle.

import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { renderWithProviders } from "@localcamera/shared/test";
import { useEffect } from "react";
import { describe, expect, it, vi } from "vitest";
import {
  type ConnectionChangedEvent,
  type ErrorEvent,
  type LocalCameraApi,
  type StoredKnownPc,
  type StatsEvent,
  type StateChangedEvent,
} from "@/plugins/LocalCamera";
import { AppProvider, useApp } from "@/state/AppProvider";
import { setPendingDeepLink } from "@/deep-link";
import { initialState, targetFromRawUri, type Target } from "@/state/connection";

const TOKEN = "0123456789abcdef0123456789abcdef";
const URI = `localcamera://connect?v=1&hosts=192.168.0.10&port=48650&token=${TOKEN}&pc=11111111-1111-4111-8111-111111111111&name=PC`;

type EventName = "connectionChanged" | "stateChanged" | "stats" | "error";
type AnyEvent = ConnectionChangedEvent | StateChangedEvent | StatsEvent | ErrorEvent;

function makeFakePlugin(): {
  api: LocalCameraApi;
  connect: ReturnType<typeof vi.fn>;
  disconnect: ReturnType<typeof vi.fn>;
  getKnownPcs: ReturnType<typeof vi.fn>;
  forgetPc: ReturnType<typeof vi.fn>;
  emit: (name: EventName, e: AnyEvent) => void;
  hasConnectionListener: () => boolean;
} {
  const buckets: Record<EventName, Set<(e: AnyEvent) => void>> = {
    connectionChanged: new Set(),
    stateChanged: new Set(),
    stats: new Set(),
    error: new Set(),
  };
  const connect = vi.fn(async (): Promise<void> => {});
  const disconnect = vi.fn(async (): Promise<void> => {});
  const getKnownPcs = vi.fn(async (): Promise<{ pcs: StoredKnownPc[] }> => ({ pcs: [] }));
  const forgetPc = vi.fn(async (_options: { pcId: string }): Promise<void> => {});
  const api = {
    getCapabilities: async (): Promise<{ caps: unknown }> => ({ caps: {} }),
    connect,
    disconnect,
    getState: async (): Promise<{ state: string }> => ({ state: "" }),
    getKnownPcs,
    forgetPc,
    addListener: async (name: EventName, fn: (e: AnyEvent) => void) => {
      buckets[name].add(fn);
      return {
        remove: async () => {
          buckets[name].delete(fn);
        },
      };
    },
  } as LocalCameraApi;
  return {
    api,
    connect,
    disconnect,
    getKnownPcs,
    forgetPc,
    hasConnectionListener: () => buckets.connectionChanged.size > 0,
    emit: (name, e) => {
      for (const fn of buckets[name]) fn(e);
    },
  };
}

/** Dispara CONNECT uma vez e oferece o botão que a UI usa para voltar ao idle. */
function ConnectProbe({ target }: { target: Target }) {
  const { dispatch } = useApp();
  useEffect(() => {
    dispatch({ type: "CONNECT", target, at: 1 });
  }, [dispatch, target]);
  return (
    <button type="button" onClick={() => dispatch({ type: "GO_HOME" })}>
      home
    </button>
  );
}

function KnownPcsProbe() {
  const { state, dispatch, knownPcsLoading } = useApp();
  return (
    <>
      <p data-testid="known-pcs-loading">{knownPcsLoading ? "loading" : "ready"}</p>
      <p data-testid="known-pc-names">{state.knownPcs.map((pc) => pc.name).join(",")}</p>
      <button
        type="button"
        onClick={() => dispatch({ type: "KPC_FORGET", pcId: "11111111-1111-4111-8111-111111111111" })}
      >
        forget
      </button>
    </>
  );
}

describe("AppProvider — ponte nativa de conexão (F2-T13)", () => {
  it("chama connect(hosts, port, token) ao entrar em connecting", async () => {
    const fake = makeFakePlugin();
    const target = targetFromRawUri(URI);

    await act(async () => {
      await renderWithProviders(
        <AppProvider initialState={initialState} localCamera={fake.api}>
          <ConnectProbe target={target} />
        </AppProvider>,
      );
    });

    await waitFor(() =>
      expect(fake.connect).toHaveBeenCalledWith({
        hosts: ["192.168.0.10"],
        port: 48650,
        token: TOKEN,
      }),
    );
  });

  it("conecta quando um novo deep link chega com o app já aberto", async () => {
    const fake = makeFakePlugin();

    await act(async () => {
      await renderWithProviders(
        <AppProvider initialState={initialState} localCamera={fake.api}>
          <div />
        </AppProvider>,
      );
    });

    await act(async () => {
      setPendingDeepLink(URI);
    });

    await waitFor(() =>
      expect(fake.connect).toHaveBeenCalledWith({
        hosts: ["192.168.0.10"],
        port: 48650,
        token: TOKEN,
      }),
    );
  });

  it("chama disconnect quando a UI sai da transmissão para idle", async () => {
    const fake = makeFakePlugin();
    const target = targetFromRawUri(URI);

    await act(async () => {
      await renderWithProviders(
        <AppProvider initialState={initialState} localCamera={fake.api}>
          <ConnectProbe target={target} />
        </AppProvider>,
      );
    });
    await waitFor(() => expect(fake.connect).toHaveBeenCalled());

    await act(async () => {
      fake.emit("connectionChanged", { state: "streaming" });
    });
    fireEvent.click(screen.getByRole("button", { name: "home" }));

    await waitFor(() => expect(fake.disconnect).toHaveBeenCalled());
  });

  it("carrega PCs persistidos e esquece usando o pcId nativo", async () => {
    const fake = makeFakePlugin();
    fake.getKnownPcs.mockResolvedValue({
      pcs: [
        {
          pcId: "11111111-1111-4111-8111-111111111111",
          name: "PC salvo",
          hosts: ["192.168.0.10"],
          port: 48650,
          lastAccessed: 123,
        },
      ],
    });

    await act(async () => {
      await renderWithProviders(
        <AppProvider localCamera={fake.api}>
          <KnownPcsProbe />
        </AppProvider>,
      );
    });
    expect(await screen.findByText("PC salvo")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "forget" }));
    await waitFor(() =>
      expect(fake.forgetPc).toHaveBeenCalledWith({ pcId: "11111111-1111-4111-8111-111111111111" }),
    );
    await waitFor(() => expect(screen.getByTestId("known-pc-names").textContent).toBe(""));
  });

  it("mostra o estado de carregamento enquanto consulta o PairingStore", async () => {
    const fake = makeFakePlugin();
    let finishLoad!: (value: { pcs: StoredKnownPc[] }) => void;
    fake.getKnownPcs.mockReturnValue(new Promise((resolve) => { finishLoad = resolve; }));

    await act(async () => {
      await renderWithProviders(
        <AppProvider localCamera={fake.api}>
          <KnownPcsProbe />
        </AppProvider>,
      );
    });
    expect(screen.getByTestId("known-pcs-loading").textContent).toBe("loading");

    await act(async () => finishLoad({ pcs: [] }));
    expect(screen.getByTestId("known-pcs-loading").textContent).toBe("ready");
  });

  it("passa o pcId ao plugin para reconectar com a chave salva", async () => {
    const fake = makeFakePlugin();
    const target: Target = {
      name: "PC salvo",
      host: "192.168.0.10",
      port: 48650,
      rawUri: null,
      token: null,
      pcId: "11111111-1111-4111-8111-111111111111",
    };

    await act(async () => {
      await renderWithProviders(
        <AppProvider localCamera={fake.api}>
          <ConnectProbe target={target} />
        </AppProvider>,
      );
    });

    await waitFor(() => expect(fake.connect).toHaveBeenCalledWith({ pcId: target.pcId }));
  });

  it("atualiza a lista nativa depois do WELCOME", async () => {
    const fake = makeFakePlugin();
    fake.getKnownPcs
      .mockResolvedValueOnce({ pcs: [] })
      .mockResolvedValue({
        pcs: [
          {
            pcId: "11111111-1111-4111-8111-111111111111",
            name: "PC após parear",
            hosts: ["192.168.0.10"],
            port: 48650,
            lastAccessed: 456,
          },
        ],
      });
    const target = targetFromRawUri(URI);

    await act(async () => {
      await renderWithProviders(
        <AppProvider localCamera={fake.api}>
          <ConnectProbe target={target} />
          <KnownPcsProbe />
        </AppProvider>,
      );
    });
    await waitFor(() => expect(fake.hasConnectionListener()).toBe(true));

    await act(async () => fake.emit("connectionChanged", { state: "streaming" }));
    expect(await screen.findByText("PC após parear")).toBeInTheDocument();
    expect(fake.getKnownPcs).toHaveBeenCalledTimes(2);
  });
});
