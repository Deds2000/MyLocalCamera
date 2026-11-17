import type { ConnectionEvent, ConnectionInfo, TauriClient } from "@/lib/tauri";
import type { AppGet, AppSet } from "./types";

export type DesktopConnectionState = "waiting" | "pairing" | "connected" | "reconnecting" | "error";

export interface ConnectionSnapshot {
  state: DesktopConnectionState;
  device: { deviceId: string; name: string | null; model: string | null; ip: string | null } | null;
  transport: "tcp" | null;
  errorCode: string | null;
}

export interface ConnectionSlice {
  connection: ConnectionSnapshot;
  connectionInfo: ConnectionInfo | null;
  connectionInfoLoading: boolean;
  connectionInfoErrorCode: string | null;
  applyConnectionEvent: (event: ConnectionEvent) => void;
  setConnectionError: (code: string) => void;
  loadConnectionInfo: () => Promise<void>;
  regeneratePairingToken: () => Promise<void>;
}

function mapConnectionState(state: ConnectionEvent["state"]): DesktopConnectionState {
  switch (state) {
    case "pending":
      return "pairing";
    case "connecting":
      return "pairing";
    case "streaming":
    case "audioConnected":
      return "connected";
    case "reconnecting":
      return "reconnecting";
    case "error":
      return "error";
    case "listening":
    case "altPort":
    case "idle":
      return "waiting";
  }
}

export function createConnectionSlice(
  set: AppSet,
  get: AppGet,
  client: TauriClient,
): ConnectionSlice {
  const loadInfo = async (): Promise<boolean> => {
    if (get().connectionInfoLoading) return false;
    set({ connectionInfoLoading: true, connectionInfoErrorCode: null });
    try {
      const connectionInfo = await client.getConnectionInfo();
      set({ connectionInfo, connectionInfoLoading: false, connectionInfoErrorCode: null });
      return true;
    } catch {
      set({ connectionInfoLoading: false, connectionInfoErrorCode: "CONNECTION_INFO_FAILED" });
      return false;
    }
  };

  return {
    connection: {
      state: "waiting",
      device: null,
      transport: null,
      errorCode: null,
    },
    connectionInfo: null,
    connectionInfoLoading: false,
    connectionInfoErrorCode: null,
    applyConnectionEvent: (event) =>
      set({
        connection: {
          state: mapConnectionState(event.state),
          device: event.deviceId
            ? { deviceId: event.deviceId, name: event.name, model: event.model, ip: event.ip }
            : null,
          transport: event.transport,
          errorCode: event.errorCode,
        },
      }),
    setConnectionError: (code) =>
      set((state) => ({
        connection: { ...state.connection, state: "error", errorCode: code },
      })),
    loadConnectionInfo: async () => {
      await loadInfo();
    },
    regeneratePairingToken: async () => {
      try {
        await client.regenerateToken();
      } catch {
        set({ connectionInfoErrorCode: "QR_REGENERATE_FAILED" });
        return;
      }
      const refreshed = await loadInfo();
      if (!refreshed) set({ connectionInfoErrorCode: "QR_REGENERATE_FAILED" });
    },
  };
}
