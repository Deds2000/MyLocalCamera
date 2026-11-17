import { describe, expect, it, vi } from "vitest";
import { createMockTauriAdapter, createTauriClient, tauri, type TauriAdapter } from "./tauri";

describe("LocalCamera Tauri bridge (F3-T13)", () => {
  it("provides a valid pairing URI in the web mock", async () => {
    const info = await tauri.getConnectionInfo();
    const pairing = new URL(info.qrcodeUri);

    expect(info.port).toBe(48650);
    expect(info.effectivePort).toBe(48650);
    expect(pairing.protocol).toBe("localcamera:");
    expect(pairing.searchParams.get("token")).toMatch(/^[0-9a-f]{32}$/);
    expect(info.pairCodeMasked).toBe("0000••••");
  });

  it("injects an adapter and forwards typed command arguments", async () => {
    const invoke = vi.fn();
    const adapter: TauriAdapter = {
      invoke: async <T>(command: string, args?: Record<string, unknown>) => {
        invoke(command, args);
        return undefined as T;
      },
      listen: async () => () => undefined,
    };
    const client = createTauriClient(adapter);

    await client.approve(42, "always");
    expect(invoke).toHaveBeenCalledWith("approve", { requestId: 42, mode: "always" });
  });

  it("delivers pending approval mock events and supports unsubscribe", async () => {
    const adapter = createMockTauriAdapter();
    const client = createTauriClient(adapter);
    const onPending = vi.fn();
    const unlisten = await client.onPendingApproval(onPending);
    const pending = {
      requestId: 7,
      deviceId: "device-7",
      name: "Phone",
      model: "Camera",
      ip: "192.168.1.20",
    };

    adapter.emit("trust://pending", pending);
    expect(onPending).toHaveBeenCalledWith(pending);

    unlisten();
    adapter.emit("trust://pending", { ...pending, requestId: 8 });
    expect(onPending).toHaveBeenCalledTimes(1);
  });
});
