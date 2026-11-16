import type { ConnectionEvent, PendingApprovalEvent, TauriClient, TrustedDevice } from "@/lib/tauri";
import type { AppGet, AppSet } from "./types";

export type TrustAction = { requestId: number; kind: "approve" | "reject" };

export interface TrustSnapshot {
  pending: PendingApprovalEvent | null;
  trusted: TrustedDevice[];
  pendingAction: TrustAction | null;
  errorCode: string | null;
}

export interface TrustSlice {
  trust: TrustSnapshot;
  applyPendingApproval: (event: PendingApprovalEvent) => void;
  confirmTrustFromConnection: (event: ConnectionEvent) => void;
  loadTrustedDevices: () => Promise<void>;
  approveTrust: (requestId: number, mode: "once" | "always") => Promise<void>;
  rejectTrust: (requestId: number) => Promise<void>;
  revokeTrustedDevice: (deviceId: string) => Promise<void>;
}

export function createTrustSlice(set: AppSet, get: AppGet, client: TauriClient): TrustSlice {
  const runTrustAction = async (
    requestId: number,
    kind: TrustAction["kind"],
    run: () => Promise<void>,
  ) => {
    const trust = get().trust;
    if (trust.pending?.requestId !== requestId) return;
    set({ trust: { ...trust, pendingAction: { requestId, kind }, errorCode: null } });
    try {
      await run();
      // Keep the approval visible until the matching session event confirms it.
    } catch {
      const current = get().trust;
      set({
        trust: {
          ...current,
          pendingAction: null,
          errorCode: kind === "approve" ? "APPROVE_FAILED" : "REJECT_FAILED",
        },
      });
    }
  };

  return {
    trust: { pending: null, trusted: [], pendingAction: null, errorCode: null },
    applyPendingApproval: (event) => set((state) => ({
      trust: { ...state.trust, pending: event, pendingAction: null, errorCode: null },
    })),
    confirmTrustFromConnection: (event) => {
      const trust = get().trust;
      if (!trust.pending) return;
      const sameDevice = event.deviceId === trust.pending.deviceId;
      const approved = sameDevice && (event.state === "streaming" || event.state === "audioConnected");
      const rejected = trust.pendingAction?.kind === "reject" && event.state === "idle";
      const failed = sameDevice && event.state === "error";
      if (approved || rejected || failed) {
        set({ trust: { ...trust, pending: null, pendingAction: null } });
        if (approved) void get().loadTrustedDevices();
      }
    },
    loadTrustedDevices: async () => {
      try {
        const trusted = await client.listTrusted();
        set((state) => ({ trust: { ...state.trust, trusted, errorCode: null } }));
      } catch {
        set((state) => ({ trust: { ...state.trust, errorCode: "TRUST_LIST_FAILED" } }));
      }
    },
    approveTrust: (requestId, mode) => runTrustAction(
      requestId,
      "approve",
      () => client.approve(requestId, mode),
    ),
    rejectTrust: (requestId) => runTrustAction(
      requestId,
      "reject",
      () => client.reject(requestId),
    ),
    revokeTrustedDevice: async (deviceId) => {
      try {
        const revoked = await client.revokeTrusted(deviceId);
        if (!revoked) {
          set((state) => ({ trust: { ...state.trust, errorCode: "TRUST_REVOKE_FAILED" } }));
          return;
        }
        const trusted = await client.listTrusted();
        set((state) => ({ trust: { ...state.trust, trusted, errorCode: null } }));
      } catch {
        set((state) => ({ trust: { ...state.trust, errorCode: "TRUST_REVOKE_FAILED" } }));
      }
    },
  };
}
