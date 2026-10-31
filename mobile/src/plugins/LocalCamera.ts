// F2-T13 — Wrapper tipado do plugin Capacitor `LocalCamera`.
//
// Ponte JS ⇄ Kotlin (lado Kotlin: `app/plugin/LocalCameraPlugin.kt`,
// contrato auditado). 4 métodos + 4 eventos, nomes fixos (não renomear):
// - métodos: `getCapabilities`, `connect`, `disconnect`, `getState`;
// - eventos: `connectionChanged`, `stateChanged`, `stats` (≤ 2 Hz) e
//   `error` (**só códigos** — nunca texto da rede).
//
// `LocalCameraApi` é a interface injetável que o `AppProvider` consome via
// a prop opcional `localCamera` (mesmo padrão do `qrScanner`): nos testes
// injeta-se um fake (ver `AppProvider.stats.test.tsx`); em produção,
// `getLocalCamera()` devolve o plugin só quando ele existe no app nativo.
//
// Segredos: o `token` das opções de `connect` só é usado para invocar o
// plugin (a URI é montada no Kotlin, via intent interno) — nunca logado
// nem guardado aqui.

import { Capacitor, registerPlugin, type PluginListenerHandle } from "@capacitor/core";

/* -------------------------------- eventos ------------------------------- */

/** Máquina §12 — mesmo vocabulário do reducer local (`state/connection.ts`). */
export type ConnectionState =
  | "idle"
  | "connecting"
  | "pending"
  | "streaming"
  | "reconnecting"
  | "error";

export interface ConnectionChangedEvent {
  state: ConnectionState;
  /** Timeout de aprovação (segundos) — só quando `state === "pending"`. */
  timeoutS?: number;
  /** Nome do PC — só quando `state === "streaming"`. */
  pcName?: string;
}

export interface StateChangedEvent {
  /** Último JSON `STATE` com `rev` (F2-T11); `""` sem sessão ativa. */
  state: string;
}

export interface StatsEvent {
  fps: number;
  kbps: number;
  queueMs: number;
  rtt: number;
}

export interface ErrorEvent {
  /** Código do erro (nunca texto da rede); a UI mapeia para i18n por código. */
  code: string;
}

/* -------------------------------- opções -------------------------------- */

/** Pareamento (QR/deep link) ou PC conhecido (`pcId` — a `deviceKey` vem
 *  do `PairingStore` no Kotlin, F2-T05). */
export type ConnectOptions =
  | { hosts: string[]; port: number; token: string }
  | { pcId: string };

/* ------------------------------- interface ----------------------------- */

export interface LocalCameraApi {
  /** `{caps}` — sonda do aparelho (F2-T06, JSON `caps` do HELLO). */
  getCapabilities(): Promise<{ caps: unknown }>;
  /**
   * Inicia a sessão. Recusas (códigos): `invalid_args`, `unknown_pc`,
   * `no_device_key`.
   */
  connect(opts: ConnectOptions): Promise<void>;
  /** Para a sessão (BYE + teardown no serviço). Idempotente. */
  disconnect(): Promise<void>;
  /** `{state}` — último JSON `STATE` com `rev`. */
  getState(): Promise<{ state: string }>;
  addListener(
    eventName: "connectionChanged",
    listenerFunc: (event: ConnectionChangedEvent) => void,
  ): Promise<PluginListenerHandle>;
  addListener(
    eventName: "stateChanged",
    listenerFunc: (event: StateChangedEvent) => void,
  ): Promise<PluginListenerHandle>;
  addListener(
    eventName: "stats",
    listenerFunc: (event: StatsEvent) => void,
  ): Promise<PluginListenerHandle>;
  addListener(
    eventName: "error",
    listenerFunc: (event: ErrorEvent) => void,
  ): Promise<PluginListenerHandle>;
}

export const LocalCamera: LocalCameraApi = registerPlugin<LocalCameraApi>("LocalCamera");

/**
 * Plugin só quando ele existe de fato no app nativo (registrado no
 * `capacitor.plugins.json`, regenerado pelo `cap sync` — não editar à mão).
 * Em web/jsdom (testes) devolve `null`: o `AppProvider` segue usando só o
 * reducer local.
 */
export function getLocalCamera(): LocalCameraApi | null {
  return Capacitor.isPluginAvailable("LocalCamera") ? LocalCamera : null;
}
