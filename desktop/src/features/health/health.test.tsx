import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "@localcamera/shared/test";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createMockTauriAdapter, createTauriClient, type HealthInfo } from "@/lib/tauri";
import { createAppStore } from "@/store";
import { HealthBar } from "./HealthBar";

const info = (state: HealthInfo["server"]["state"]): HealthInfo => ({
  server: { state, port: 48650, effectivePort: state === "altPort" ? 48651 : 48650 },
  stubs: ["em breve"],
});

function renderHealth(state: HealthInfo["server"]["state"] | null = "listening") {
  const adapter = createMockTauriAdapter();
  const store = createAppStore(createTauriClient(adapter));
  if (state)
    act(() =>
      store.setState({
        health: { data: info(state), loading: false, errorCode: null, restarting: false },
      }),
    );
  return { store, adapter, render: renderWithProviders(<HealthBar store={store} />) };
}

describe("HealthBar", () => {
  beforeEach(() => {
    localStorage.clear();
  });
  it.each([
    ["listening", "Servidor :48650", "Servidor :48650"],
    ["altPort", ":48651 (porta alterada)", "Servidor :48651 (porta alterada)"],
    ["failed", "Servidor parado", "Servidor parado"],
  ] as const)("renderiza servidor %s", async (state, text, accessibleName) => {
    await renderHealth(state).render;
    expect(screen.getByText(text)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: accessibleName })).toBeInTheDocument();
  });
  it("exibe quatro itens em breve sem torná-los interativos", async () => {
    await renderHealth().render;
    expect(screen.getAllByText(/Em breve/)).toHaveLength(4);
    for (const kind of ["discovery", "vcam", "mic", "adb"]) {
      expect(document.querySelector(`[data-kind="${kind}"]`)).not.toHaveProperty(
        "tagName",
        "BUTTON",
      );
    }
  });
  it("abre detalhe de escuta e fecha com Escape devolvendo foco", async () => {
    const user = userEvent.setup();
    await renderHealth().render;
    const trigger = screen.getByRole("button", { name: /^Servidor/ });
    await user.click(trigger);
    expect(screen.getByRole("dialog", { name: "Detalhes do servidor" })).toHaveTextContent(
      "porta 48650",
    );
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(trigger).toHaveFocus();
  });
  it("fecha ao clicar fora e devolve foco ao botão Servidor", async () => {
    const user = userEvent.setup();
    await renderHealth().render;
    const trigger = screen.getByRole("button", { name: /^Servidor/ });
    await user.click(trigger);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    await user.click(document.body);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(trigger).toHaveFocus();
  });
  it("mostra ajustes desabilitados e tooltip no estado de porta alternativa", async () => {
    const user = userEvent.setup();
    await renderHealth("altPort").render;
    await user.click(screen.getByRole("button", { name: /^Servidor/ }));
    expect(screen.getByRole("button", { name: "Abrir Ajustes" })).toBeDisabled();
    await user.hover(screen.getByText("Abrir Ajustes").parentElement!);
    expect(await screen.findByText("Os Ajustes estarão disponíveis em breve.")).toBeInTheDocument();
  });
  it("aciona retry e apresenta progresso e falha sem inventar estado", async () => {
    const user = userEvent.setup();
    const adapter = createMockTauriAdapter();
    const store = createAppStore(createTauriClient(adapter));
    act(() =>
      store.setState({
        health: { data: info("failed"), loading: false, errorCode: null, restarting: false },
      }),
    );
    let finishRestart!: () => void;
    const restart = vi.fn(async () => {
      store.setState((state) => ({ health: { ...state.health, restarting: true } }));
      await new Promise<void>((resolve) => {
        finishRestart = resolve;
      });
      store.setState((state) => ({
        health: { ...state.health, restarting: false, errorCode: "HEALTH_RESTART_FAILED" },
      }));
    });
    store.setState({ restartServer: restart });
    await renderWithProviders(<HealthBar store={store} />);
    await user.click(screen.getByRole("button", { name: /^Servidor/ }));
    await user.click(screen.getByRole("button", { name: "Tentar novamente" }));
    expect(restart).toHaveBeenCalledOnce();
    expect(await screen.findByRole("button", { name: "Reiniciando…" })).toBeDisabled();
    await act(async () => {
      finishRestart();
    });
    expect(await screen.findByRole("status")).toHaveTextContent("Não foi possível reiniciar");
  });
  it("recarrega health em vez de reiniciar quando ainda não há dados", async () => {
    const user = userEvent.setup();
    const adapter = createMockTauriAdapter();
    const store = createAppStore(createTauriClient(adapter));
    const loadHealth = vi.fn(async () => {});
    const restart = vi.fn(async () => {});
    store.setState({
      health: { data: null, loading: true, errorCode: null, restarting: false },
      loadHealth,
      restartServer: restart,
    });
    await renderWithProviders(<HealthBar store={store} />);
    await user.click(screen.getByRole("button", { name: /^Servidor/ }));
    await user.click(screen.getByRole("button", { name: "Tentar novamente" }));
    expect(loadHealth).toHaveBeenCalledOnce();
    expect(restart).not.toHaveBeenCalled();
  });

  it("mantém rodapé de 32px e popover visível fora dos limites da barra", async () => {
    await renderHealth().render;
    const bar = screen.getByTestId("health-bar");
    expect(bar).toHaveClass("h-8", "min-w-0");
    expect(bar).not.toHaveClass("overflow-hidden");
    expect(bar.querySelectorAll("[data-kind]")).toHaveLength(5);
    const shellFooter = document.createElement("footer");
    shellFooter.className = "h-8 shrink-0 border-t border-glass-border bg-raised";
    expect(shellFooter).not.toHaveClass("overflow-hidden");
    await userEvent.setup().click(screen.getByRole("button", { name: /^Servidor/ }));
    const popover = screen.getByTestId("health-popover");
    expect(popover.parentElement).toBe(bar.querySelector(".relative"));
    expect(popover).toHaveClass("absolute", "bottom-full");
  });
});
