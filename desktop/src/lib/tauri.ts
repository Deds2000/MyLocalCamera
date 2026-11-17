import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";

export interface NetworkInterface {
  name: string;
  ip: string;
  isVirtual: boolean;
}

export interface ConnectionInfo {
  port: number;
  effectivePort: number;
  interfaces: NetworkInterface[];
  qrcodeUri: string;
  pairCodeMasked: string;
  pcId: string;
  name: string;
}

export interface TrustedDevice {
  deviceId: string;
  name: string;
  model: string | null;
  lastSeen: number;
  lastIp: string;
}

export interface HealthInfo {
  server: {
    state: "listening" | "altPort" | "failed";
    port: number;
    effectivePort: number;
  };
  stubs: string[];
}

export type ConnectionState =
  | "listening"
  | "altPort"
  | "connecting"
  | "streaming"
  | "pending"
  | "idle"
  | "reconnecting"
  | "error"
  | "audioConnected";

export interface ConnectionEvent {
  state: ConnectionState;
  sessionId: string | null;
  deviceId: string | null;
  name: string | null;
  model: string | null;
  transport: "tcp" | null;
  ip: string | null;
  errorCode: string | null;
  requestedPort: number | null;
  effectivePort: number | null;
}

export interface StatsEvent {
  sessionId: string;
  fps: number;
  bitrateKbps: number;
  rttMs: number | null;
  lost: number;
}

export interface PendingApprovalEvent {
  requestId: number;
  deviceId: string;
  name: string;
  model: string;
  ip: string;
}

export type ApprovalMode = "once" | "always";
export type EventHandler<T> = (payload: T) => void;

export interface TauriAdapter {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  listen<T>(event: string, handler: EventHandler<T>): Promise<UnlistenFn>;
}

export interface TauriClient {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  listen<T>(event: string, handler: EventHandler<T>): Promise<UnlistenFn>;
  getConnectionInfo(): Promise<ConnectionInfo>;
  regenerateToken(): Promise<string>;
  approve(requestId: number, mode: ApprovalMode): Promise<void>;
  reject(requestId: number): Promise<void>;
  listTrusted(): Promise<TrustedDevice[]>;
  revokeTrusted(deviceId: string): Promise<boolean>;
  getHealth(): Promise<HealthInfo>;
  restartServer(): Promise<void>;
  onConnection(handler: EventHandler<ConnectionEvent>): Promise<UnlistenFn>;
  onStats(handler: EventHandler<StatsEvent>): Promise<UnlistenFn>;
  onPendingApproval(handler: EventHandler<PendingApprovalEvent>): Promise<UnlistenFn>;
  onHealthChanged(handler: EventHandler<HealthInfo>): Promise<UnlistenFn>;
}

export function createTauriClient(adapter: TauriAdapter): TauriClient {
  return {
    invoke: (command, args) => adapter.invoke(command, args),
    listen: (event, handler) => adapter.listen(event, handler),
    getConnectionInfo: () => adapter.invoke("get_connection_info"),
    regenerateToken: () => adapter.invoke("regenerate_token"),
    approve: (requestId, mode) => adapter.invoke("approve", { requestId, mode }),
    reject: (requestId) => adapter.invoke("reject", { requestId }),
    listTrusted: () => adapter.invoke("list_trusted"),
    revokeTrusted: (deviceId) => adapter.invoke("revoke_trusted", { deviceId }),
    getHealth: () => adapter.invoke("get_health"),
    restartServer: () => adapter.invoke("restart_server"),
    onConnection: (handler) => adapter.listen("session://connection", handler),
    onStats: (handler) => adapter.listen("session://stats", handler),
    onPendingApproval: (handler) => adapter.listen("trust://pending", handler),
    onHealthChanged: (handler) => adapter.listen("health://changed", handler),
  };
}

export interface MockTauriAdapter extends TauriAdapter {
  emit<T>(event: string, payload: T): void;
}

export function createMockTauriAdapter(): MockTauriAdapter {
  const listeners = new Map<string, Set<EventHandler<unknown>>>();
  const adapter: MockTauriAdapter = {
    async invoke<T>(command: string): Promise<T> {
      const responses: Record<string, unknown> = {
        get_connection_info: {
          port: 48650,
          effectivePort: 48650,
          interfaces: [{ name: "mock", ip: "127.0.0.1", isVirtual: false }],
          qrcodeUri:
            "localcamera://connect?v=1&hosts=127.0.0.1&port=48650&token=00000000000000000000000000000000&pc=mock-pc&name=LocalCamera%20mock",
          pairCodeMasked: "0000••••",
          pcId: "mock-pc",
          name: "LocalCamera mock",
        },
        regenerate_token: "0000••••",
        approve: undefined,
        reject: undefined,
        list_trusted: [],
        revoke_trusted: false,
        get_health: {
          server: { state: "listening", port: 48650, effectivePort: 48650 },
          stubs: ["em breve"],
        },
        restart_server: undefined,
      };
      if (!(command in responses)) {
        throw new Error("Unsupported LocalCamera mock command");
      }
      return responses[command] as T;
    },
    async listen<T>(event: string, handler: EventHandler<T>): Promise<UnlistenFn> {
      const handlers = listeners.get(event) ?? new Set<EventHandler<unknown>>();
      handlers.add(handler as EventHandler<unknown>);
      listeners.set(event, handlers);
      return () => {
        handlers.delete(handler as EventHandler<unknown>);
        if (handlers.size === 0) listeners.delete(event);
      };
    },
    emit<T>(event: string, payload: T): void {
      for (const handler of listeners.get(event) ?? []) handler(payload);
    },
  };
  return adapter;
}

const realAdapter: TauriAdapter = {
  invoke: <T>(command: string, args?: Record<string, unknown>) =>
    tauriInvoke<T>(command, args),
  listen: async <T>(event: string, handler: EventHandler<T>) =>
    tauriListen<T>(event, (tauriEvent) => handler(tauriEvent.payload)),
};

const defaultAdapter =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
    ? realAdapter
    : createMockTauriAdapter();

export const tauri = createTauriClient(defaultAdapter);
