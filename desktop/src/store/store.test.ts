import { act, render, screen } from "@testing-library/react";
import { createElement } from "react";
import { useStore } from "zustand";
import {
  createMockTauriAdapter,
  createTauriClient,
  type ConnectionEvent,
  type StatsEvent,
} from "@/lib/tauri";
import { useTauriEvents } from "@/hooks/useTauriEvents";
import { appStore, createAppStore, selectConnectionState, selectStats } from "./index";
import { beforeEach, describe, expect, it } from "vitest";

const pendingApproval = {
  requestId: 17,
  deviceId: "phone-1",
  name: "Camera phone",
  model: "SM-G781B",
  ip: "192.168.1.20",
};

function connectionEvent(
  state: ConnectionEvent["state"],
  deviceId: string | null,
): ConnectionEvent {
  return {
    state,
    sessionId: state === "streaming" ? "session-1" : null,
    deviceId,
    name: deviceId ? "Camera phone" : null,
    model: deviceId ? "SM-G781B" : null,
    transport: state === "streaming" ? "tcp" : null,
    ip: deviceId ? "192.168.1.20" : null,
    errorCode: null,
    requestedPort: null,
    effectivePort: null,
  };
}

beforeEach(() => {
  localStorage.clear();
  appStore.setState({ settings: { ...appStore.getState().settings, language: "system" } });
});

describe("store Zustand (F3-T14a)", () => {
  it("UT-3.11 mantém seletores estreitos durante 2 Hz por 5 s", async () => {
    const adapter = createMockTauriAdapter();
    const client = createTauriClient(adapter);
    const store = createAppStore(client);
    let statsRenders = 0;
    let connectionRenders = 0;

    function StatsView() {
      const stats = useStore(store, selectStats);
      statsRenders += 1;
      return createElement("output", { "data-testid": "stats-value" }, stats?.fps ?? 0);
    }

    function ConnectionView() {
      const state = useStore(store, selectConnectionState);
      connectionRenders += 1;
      return createElement("output", { "data-testid": "connection-value" }, state);
    }

    function Events() {
      useTauriEvents(client, store);
      return null;
    }

    render(
      createElement(
        "div",
        null,
        createElement(Events),
        createElement(StatsView),
        createElement(ConnectionView),
      ),
    );
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
    const initialStatsRenders = statsRenders;
    const initialConnectionRenders = connectionRenders;

    for (let sample = 1; sample <= 10; sample += 1) {
      await act(async () => {
        const event: StatsEvent = {
          sessionId: "session-1",
          fps: sample,
          bitrateKbps: 8_000 + sample,
          rttMs: sample,
          lost: 0,
        };
        adapter.emit("session://stats", event);
        await new Promise((resolve) => setTimeout(resolve, 500));
      });
    }

    expect(screen.getByTestId("stats-value")).toHaveTextContent("10");
    expect(statsRenders).toBeGreaterThan(initialStatsRenders);
    expect(connectionRenders).toBe(initialConnectionRenders);
  }, 10_000);

  it("confirma approve somente quando chega o evento de sessão", async () => {
    const client = createTauriClient(createMockTauriAdapter());
    const store = createAppStore(client);
    store.getState().applyPendingApproval(pendingApproval);

    await store.getState().approveTrust(pendingApproval.requestId, "always");
    expect(store.getState().trust.pending).toEqual(pendingApproval);
    expect(store.getState().trust.pendingAction).toEqual({ requestId: 17, kind: "approve" });

    store.getState().confirmTrustFromConnection(connectionEvent("streaming", "other-phone"));
    expect(store.getState().trust.pending).toEqual(pendingApproval);

    store.getState().confirmTrustFromConnection(connectionEvent("streaming", "phone-1"));
    expect(store.getState().trust.pending).toBeNull();
    expect(store.getState().trust.pendingAction).toBeNull();
  });

  it("converte falha de invoke em código estável sem rejeitar a ação", async () => {
    const adapter = createMockTauriAdapter();
    const client = createTauriClient({
      invoke: async <T>(command: string, args?: Record<string, unknown>): Promise<T> => {
        if (command === "approve") throw new Error("private backend detail");
        return adapter.invoke<T>(command, args);
      },
      listen: (event, handler) => adapter.listen(event, handler),
    });
    const store = createAppStore(client);
    store.getState().applyPendingApproval(pendingApproval);

    await store.getState().approveTrust(pendingApproval.requestId, "once");

    expect(store.getState().trust.pending).toEqual(pendingApproval);
    expect(store.getState().trust.pendingAction).toBeNull();
    expect(store.getState().trust.errorCode).toBe("APPROVE_FAILED");
  });

  it("persiste idioma no slice settings e restaura ao criar outro store", () => {
    const client = createTauriClient(createMockTauriAdapter());
    const first = createAppStore(client);
    first.getState().setLanguage("en");
    expect(localStorage.getItem("lang")).toBe("en");

    const second = createAppStore(client);
    expect(second.getState().settings.language).toBe("en");
  });

  it("guarda o espelho de STATE sem aplicar reconciliação otimista", () => {
    const store = createAppStore(createTauriClient(createMockTauriAdapter()));
    store.getState().applyCameraState({
      rev: 9,
      camera: { mode: "single", active: "back" },
      video: null,
      presentation: null,
      audio: null,
    });

    expect(store.getState().cameraState).toEqual({
      rev: 9,
      camera: { mode: "single", active: "back" },
      video: null,
      presentation: null,
      audio: null,
      pending: false,
    });
  });
});
