import type { ConnectionEvent } from "@/lib/tauri";
import type { AppSet } from "./types";

export type DesktopConnectionState = "waiting" | "pairing" | "connected" | "reconnecting" | "error";

export interface ConnectionSnapshot {
  state: DesktopConnectionState;
  device: { deviceId: string; name: string | null; model: string | null; ip: string | null } | null;
  transport: "tcp" | null;
  errorCode: string | null;
}

export interface ConnectionSlice {
  connection: ConnectionSnapshot;
  applyConnectionEvent: (event: ConnectionEvent) => void;
  setConnectionError: (code: string) => void;
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

export function createConnectionSlice(set: AppSet): ConnectionSlice {
  return {
    connection: {
      state: "waiting",
      device: null,
      transport: null,
      errorCode: null,
    },
    applyConnectionEvent: (event) => set({
      connection: {
        state: mapConnectionState(event.state),
        device: event.deviceId
          ? { deviceId: event.deviceId, name: event.name, model: event.model, ip: event.ip }
          : null,
        transport: event.transport,
        errorCode: event.errorCode,
      },
    }),
    setConnectionError: (code) => set((state) => ({
      connection: { ...state.connection, state: "error", errorCode: code },
    })),
  };
}
