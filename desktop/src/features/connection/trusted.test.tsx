import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "@localcamera/shared/test";
import { describe, expect, it, vi } from "vitest";
import { createTauriClient, type TauriAdapter } from "@/lib/tauri";
import { createAppStore } from "@/store";
import { TrustedList } from "./TrustedList";
import type { TrustedDevice } from "@/lib/tauri";

const phone = {
  deviceId: "device-42",
  name: "Pixel",
  model: "Pixel 8",
  lastSeen: 1_700_000_000,
  lastIp: "192.168.1.4",
};

function setup({
  failRevoke = false,
  revokeFalse = false,
  deferRelist = false,
  failRelistTimes = 0,
  keepListedAfterRevoke = false,
} = {}) {
  let trusted = [phone];
  let releaseRelist: (() => void) | undefined;
  let revoked = false;
  let postRevokeLists = 0;
  const invoke = vi.fn(
    async (command: string, args?: Record<string, unknown>): Promise<unknown> => {
      if (command === "list_trusted") {
        if (revoked) {
          postRevokeLists += 1;
          if (deferRelist)
            await new Promise<void>((resolve) => {
              releaseRelist = resolve;
            });
          if (postRevokeLists <= failRelistTimes) throw new Error("relist failed");
        }
        return trusted;
      }
      if (command === "revoke_trusted") {
        expect(args).toEqual({ deviceId: "device-42" });
        if (failRevoke) throw new Error("failed");
        if (revokeFalse) return false;
        revoked = true;
        if (!keepListedAfterRevoke) trusted = [];
        return true;
      }
      throw new Error(`unexpected command ${command}`);
    },
  );
  const adapter: TauriAdapter = {
    invoke: <T,>(command: string, args?: Record<string, unknown>) =>
      invoke(command, args) as Promise<T>,
    async listen() {
      return () => undefined;
    },
  };
  const store = createAppStore(createTauriClient(adapter));
  return { store, invoke, finishRelist: () => releaseRelist?.() };
}

describe("TrustedList (F3-T06)", () => {
  it("UT-3.22 cancelar preserva o item; confirmar revoga o deviceId correto e relista", async () => {
    const user = userEvent.setup();
    const fixture = setup({ deferRelist: true });
    await renderWithProviders(<TrustedList store={fixture.store} />);
    await screen.findByText("Pixel");
    await user.click(screen.getByRole("button", { name: "Remover Pixel" }));
    expect(screen.getByRole("button", { name: "Cancelar" })).toHaveFocus();
    await user.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(screen.getByText("Pixel")).toBeInTheDocument();
    expect(fixture.invoke).not.toHaveBeenCalledWith("revoke_trusted", expect.anything());

    await user.click(screen.getByRole("button", { name: "Remover Pixel" }));
    await user.click(screen.getByRole("button", { name: "Confirmar" }));
    expect(await screen.findByText("Removendo…")).toBeInTheDocument();
    expect(screen.getByText("Pixel")).toBeInTheDocument();
    const revokeIndex = fixture.invoke.mock.calls.findIndex(
      ([command]) => command === "revoke_trusted",
    );
    expect(revokeIndex).toBeGreaterThanOrEqual(0);
    expect(
      fixture.invoke.mock.calls
        .slice(revokeIndex + 1)
        .some(([command]) => command === "list_trusted"),
    ).toBe(true);
    await act(async () => fixture.finishRelist());
    await waitFor(() => expect(screen.getByText("Nenhum aparelho pareado.")).toBeInTheDocument());
    expect(fixture.invoke).toHaveBeenCalledWith("revoke_trusted", { deviceId: "device-42" });
    expect(document.activeElement).toBe(document.getElementById("trusted-title"));
  });

  it("mantém a linha durante o comando e informa falha sem remover", async () => {
    const user = userEvent.setup();
    const fixture = setup({ failRevoke: true });
    await renderWithProviders(<TrustedList store={fixture.store} />);
    await screen.findByText("Pixel");
    await user.click(screen.getByRole("button", { name: "Remover Pixel" }));
    await user.click(screen.getByRole("button", { name: "Confirmar" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Não foi possível confirmar a remoção",
    );
    expect(screen.getByText("Pixel")).toBeInTheDocument();
  });

  it("não remove se revoke responder false", async () => {
    const user = userEvent.setup();
    const fixture = setup({ revokeFalse: true });
    await renderWithProviders(<TrustedList store={fixture.store} />);
    await screen.findByText("Pixel");
    await user.click(screen.getByRole("button", { name: "Remover Pixel" }));
    await user.click(screen.getByRole("button", { name: "Confirmar" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Não foi possível confirmar a remoção",
    );
    expect(screen.getByText("Pixel")).toBeInTheDocument();
    expect(fixture.invoke).toHaveBeenCalledTimes(2);
  });

  it("permite abrir e cancelar pelo teclado, restaurando foco ao trigger", async () => {
    const user = userEvent.setup();
    const fixture = setup();
    await renderWithProviders(<TrustedList store={fixture.store} />);
    await screen.findByText("Pixel");
    const trigger = screen.getByRole("button", { name: "Remover Pixel" });
    trigger.focus();
    await user.keyboard("{Enter}");
    expect(screen.getByRole("button", { name: "Cancelar" })).toHaveFocus();
    await user.keyboard("{Tab}");
    expect(screen.getByRole("button", { name: "Confirmar" })).toHaveFocus();
    await user.keyboard("{Shift>}{Tab}{/Shift}");
    expect(screen.getByRole("button", { name: "Cancelar" })).toHaveFocus();
    await user.keyboard("{Enter}");
    await waitFor(() => expect(trigger).toHaveFocus());
    expect(screen.getByText("Pixel")).toBeInTheDocument();
  });

  it("recupera com atualização após relist falho sem revogar de novo", async () => {
    const user = userEvent.setup();
    const fixture = setup({ failRelistTimes: 1 });
    await renderWithProviders(<TrustedList store={fixture.store} />);
    await screen.findByText("Pixel");
    await user.click(screen.getByRole("button", { name: "Remover Pixel" }));
    await user.click(screen.getByRole("button", { name: "Confirmar" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Não foi possível confirmar a remoção",
    );
    await user.click(screen.getByRole("button", { name: "Atualizar lista" }));
    await waitFor(() => expect(screen.getByText("Nenhum aparelho pareado.")).toBeInTheDocument());
    expect(
      fixture.invoke.mock.calls.filter(([command]) => command === "revoke_trusted"),
    ).toHaveLength(1);
    expect(document.activeElement).toBe(document.getElementById("trusted-title"));
  });

  it("mantém confirmação disponível quando refresh confirma que dispositivo continua listado", async () => {
    const user = userEvent.setup();
    const fixture = setup({ failRelistTimes: 1, keepListedAfterRevoke: true });
    await renderWithProviders(<TrustedList store={fixture.store} />);
    await screen.findByText("Pixel");
    await user.click(screen.getByRole("button", { name: "Remover Pixel" }));
    await user.click(screen.getByRole("button", { name: "Confirmar" }));
    await screen.findByRole("alert");
    await user.click(screen.getByRole("button", { name: "Atualizar lista" }));
    expect(await screen.findByRole("status")).toHaveTextContent(
      "O dispositivo ainda consta na lista",
    );
    expect(screen.getByRole("button", { name: "Confirmar" })).toBeEnabled();
    expect(screen.getByText("Pixel")).toBeInTheDocument();
    expect(
      fixture.invoke.mock.calls.filter(([command]) => command === "revoke_trusted"),
    ).toHaveLength(1);
  });

  it("mantém estado seguro se refresh falha e recupera numa nova atualização", async () => {
    const user = userEvent.setup();
    const fixture = setup({ failRelistTimes: 2 });
    await renderWithProviders(<TrustedList store={fixture.store} />);
    await screen.findByText("Pixel");
    await user.click(screen.getByRole("button", { name: "Remover Pixel" }));
    await user.click(screen.getByRole("button", { name: "Confirmar" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Não foi possível confirmar a remoção",
    );
    await user.click(screen.getByRole("button", { name: "Atualizar lista" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Não foi possível carregar os dispositivos confiáveis.",
    );
    expect(screen.getByText("Pixel")).toBeInTheDocument();
    expect(fixture.store.getState().trust.trusted).toHaveLength(1);
    expect(fixture.store.getState().trust.errorCode).toBe("TRUST_LIST_FAILED");
    await user.click(screen.getByRole("button", { name: "Atualizar lista" }));
    await waitFor(() => expect(screen.getByText("Nenhum aparelho pareado.")).toBeInTheDocument());
    expect(
      fixture.invoke.mock.calls.filter(([command]) => command === "revoke_trusted"),
    ).toHaveLength(1);
    expect(document.activeElement).toBe(document.getElementById("trusted-title"));
  });

  it("expõe carregamento e permite retentar uma falha de listagem", async () => {
    const user = userEvent.setup();
    let fail = true;
    const invoke = vi.fn(
      async (command: string, _args?: Record<string, unknown>): Promise<unknown> => {
        void _args;
        if (command === "list_trusted") {
          if (fail) throw new Error("failed");
          return [];
        }
        throw new Error("unexpected");
      },
    );
    const store = createAppStore(
      createTauriClient({
        invoke: <T,>(command: string, args?: Record<string, unknown>) =>
          invoke(command, args) as Promise<T>,
        async listen() {
          return () => undefined;
        },
      }),
    );
    await act(async () => {
      await renderWithProviders(<TrustedList store={store} />);
    });
    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent("Não foi possível carregar"),
    );
    fail = false;
    await user.click(screen.getByRole("button", { name: "Tentar de novo" }));
    await waitFor(() => expect(screen.getByText("Nenhum aparelho pareado.")).toBeInTheDocument());
  });

  it("mostra carregamento enquanto listagem está pendente e formata horário conforme idioma", async () => {
    let resolveList: ((value: TrustedDevice[]) => void) | undefined;
    const invoke = vi.fn(
      async (command: string, _args?: Record<string, unknown>): Promise<unknown> => {
        void _args;
        if (command === "list_trusted")
          return new Promise<TrustedDevice[]>((resolve) => {
            resolveList = resolve;
          });
        throw new Error("unexpected");
      },
    );
    const store = createAppStore(
      createTauriClient({
        invoke: <T,>(command: string, args?: Record<string, unknown>) =>
          invoke(command, args) as Promise<T>,
        async listen() {
          return () => undefined;
        },
      }),
    );
    const { rerender } = await renderWithProviders(<TrustedList store={store} />, "en");
    expect(screen.getByRole("status")).toHaveTextContent("Loading trusted devices");
    await act(async () => resolveList?.([phone]));
    expect(await screen.findByText(/Last seen .* ago/)).toBeInTheDocument();
    rerender(<TrustedList store={store} />);
  });
});
