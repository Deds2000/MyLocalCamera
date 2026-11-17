import { act, render } from "@testing-library/react";
import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { createTauriClient, type HealthInfo, type TauriAdapter } from "@/lib/tauri";
import { createAppStore } from "@/store";
import { useTauriEvents } from "./useTauriEvents";

describe("useTauriEvents health subscription", () => {
  it("registers five listeners, applies health and removes all on unmount", async () => {
    const callbacks = new Map<string, Set<(payload: unknown) => void>>();
    const calls: string[] = [];
    const adapter: TauriAdapter = {
      invoke: async <T,>(command: string): Promise<T> => {
        if (command === "get_health")
          return {
            server: { state: "listening", port: 48650, effectivePort: 48650 },
            stubs: [],
          } as T;
        if (command === "list_trusted") return [] as T;
        if (command === "get_connection_info") throw new Error("unused");
        return undefined as T;
      },
      listen: async <T,>(event: string, handler: (payload: T) => void) => {
        calls.push(event);
        const handlers = callbacks.get(event) ?? new Set();
        handlers.add(handler as (payload: unknown) => void);
        callbacks.set(event, handlers);
        return () => handlers.delete(handler as (payload: unknown) => void);
      },
    };
    const store = createAppStore(createTauriClient(adapter));
    function Events() {
      useTauriEvents(createTauriClient(adapter), store);
      return null;
    }
    const view = render(createElement(Events));
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(calls).toHaveLength(5);
    expect(calls[0]).toBe("health://changed");
    const changed: HealthInfo = {
      server: { state: "failed", port: 48650, effectivePort: 0 },
      stubs: [],
    };
    act(() => {
      for (const handler of callbacks.get("health://changed") ?? []) handler(changed);
    });
    expect(store.getState().health.data).toEqual(changed);
    view.unmount();
    expect([...callbacks.values()].every((handlers) => handlers.size === 0)).toBe(true);
  });
});
