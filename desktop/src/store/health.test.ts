import { describe, expect, it } from "vitest";
import { createMockTauriAdapter, createTauriClient, type HealthInfo } from "@/lib/tauri";
import { createAppStore } from "./index";

const listening: HealthInfo = {
  server: { state: "listening", port: 48650, effectivePort: 48650 },
  stubs: ["em breve"],
};
const failed: HealthInfo = {
  server: { state: "failed", port: 48650, effectivePort: 0 },
  stubs: ["em breve"],
};

describe("health slice", () => {
  it("loads initial health and converts load errors to a stable code", async () => {
    const adapter = createMockTauriAdapter();
    const store = createAppStore(createTauriClient(adapter));
    await store.getState().loadHealth();
    expect(store.getState().health.data?.server.state).toBe("listening");
    expect(store.getState().health.loading).toBe(false);

    const failing = createAppStore(createTauriClient({
      invoke: async <T>(command: string): Promise<T> => {
        if (command === "get_health") throw new Error("private details");
        return undefined as T;
      },
      listen: (event, handler) => adapter.listen(event, handler),
    }));
    await failing.getState().loadHealth();
    expect(failing.getState().health.errorCode).toBe("HEALTH_LOAD_FAILED");
    expect(failing.getState().health.loading).toBe(false);
  });

  it("applies health events and prevents an older load from overwriting one", async () => {
    let resolveHealth!: (value: HealthInfo) => void;
    const adapter = createMockTauriAdapter();
    const client = createTauriClient({
      invoke: async <T>(command: string): Promise<T> => {
        if (command === "get_health") return await new Promise<HealthInfo>((resolve) => {
          resolveHealth = resolve;
        }) as T;
        return adapter.invoke<T>(command);
      },
      listen: (event, handler) => adapter.listen(event, handler),
    });
    const store = createAppStore(client);
    const loading = store.getState().loadHealth();
    store.getState().applyHealthEvent(failed);
    resolveHealth(listening);
    await loading;
    expect(store.getState().health.data).toEqual(failed);
    expect(store.getState().health.loading).toBe(false);
  });

  it("restart succeeds without optimistic health, and reports failures stably", async () => {
    const adapter = createMockTauriAdapter();
    let resolveRestart!: () => void;
    const client = createTauriClient({
      invoke: async <T>(command: string): Promise<T> => {
        if (command === "get_health") return listening as T;
        if (command === "restart_server") return await new Promise<void>((resolve) => {
          resolveRestart = resolve;
        }) as T;
        return adapter.invoke<T>(command);
      },
      listen: (event, handler) => adapter.listen(event, handler),
    });
    const store = createAppStore(client);
    store.getState().applyHealthEvent(failed);
    const restarting = store.getState().restartServer();
    expect(store.getState().health.restarting).toBe(true);
    expect(store.getState().health.data).toEqual(failed);
    resolveRestart();
    await restarting;
    expect(store.getState().health.restarting).toBe(false);
    expect(store.getState().health.data).toEqual(failed);

    const failedClient = createTauriClient({
      invoke: async <T>(command: string): Promise<T> => {
        if (command === "restart_server") throw new Error("private details");
        return adapter.invoke<T>(command);
      },
      listen: (event, handler) => adapter.listen(event, handler),
    });
    const failedStore = createAppStore(failedClient);
    await failedStore.getState().restartServer();
    expect(failedStore.getState().health.errorCode).toBe("HEALTH_RESTART_FAILED");
    expect(failedStore.getState().health.restarting).toBe(false);
  });
});
