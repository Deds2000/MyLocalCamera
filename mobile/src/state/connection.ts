// F2-T03 — Máquina de estados da conexão (mínimo da Fase 2, cf. protocolo §12).
//
// O estado vem de um reducer local: o fio para o plugin Kotlin (F2-T13)
// despachará os MESMOS a partir dos eventos nativos
// (connectionChanged, stateChanged, stats, error). Nada de texto da rede
// entra na UI: `reason` é sempre mapeado por código (UT-2.09).

import { parsePairingUri } from "@localcamera/shared/protocol";

/* ------------------------------ tipos ------------------------------ */

export type PermissionKind = "camera" | "mic" | "notifications";
export type PermissionStatus = "unknown" | "granted" | "denied";

export interface Permissions {
  camera: PermissionStatus;
  mic: PermissionStatus;
  notifications: PermissionStatus;
}

/** Alvo de conexão exibido na UI. `rawUri` carrega o token (segredo):
 *  fica apenas em memória — nunca logar nem renderizar. */
export interface Target {
  /** Nome para exibição (nunca contém segredo). */
  name: string;
  host: string | null;
  port: number | null;
  /** URI bruta do QR/deep link (`null` quando o alvo foi digitado). */
  rawUri: string | null;
  /** Código de pareamento digitado na tela Manual (`null` em QR/deep link). */
  token: string | null;
}

export interface KnownPc {
  name: string;
  host: string;
  port: number;
  lastSeenMs: number | null;
}

export interface StreamStats {
  fps: number | null;
  bitrateKbps: number | null;
  rttMs: number | null;
  dropped: number;
}

export interface StreamWarnings {
  slow: boolean;
  hot: boolean;
  battery: boolean;
}

export type CameraMode = "back" | "front" | "dual";

export interface StreamInfo {
  camera: CameraMode;
  mic: boolean;
  mirror: boolean;
  rotation: number;
  caps: { concurrent: boolean };
  stats: StreamStats;
  warnings: StreamWarnings;
}

export type Connection =
  | { kind: "idle" }
  | { kind: "connecting"; target: Target; at: number }
  | { kind: "pending"; target: Target; timeoutS: number }
  | { kind: "streaming"; target: Target }
  | { kind: "reconnecting"; target: Target }
  | {
      kind: "error";
      reason: string;
      target: Target | null;
      versions?: { phone: string; pc: string };
      versionSide?: "phone" | "pc";
    };

export type Route = "home" | "scan" | "manual" | "knownPcs";
export type Sheet = "settings";

export interface Settings {
  /** "Continuar transmitindo com a tela apagada" (padrão ligado, §3.11). */
  background: boolean;
  keepScreenOn: boolean;
}

export interface AppState {
  connection: Connection;
  permissions: Permissions;
  /** `true` após o usuário concluir a tela de permissões (§3.3). */
  permissionsDone: boolean;
  /** Navegação local (só tem efeito em `idle`). */
  route: Route;
  /** Sheet aberto (o botão voltar o fecha — §3.1). */
  sheet: Sheet | null;
  knownPcs: KnownPc[];
  settings: Settings;
  stream: StreamInfo;
}

/* ------------------------------ ações ------------------------------ */

export type AppAction =
  | { type: "PERMISSION"; kind: PermissionKind; status: PermissionStatus }
  | { type: "PERMISSIONS_DONE" }
  | { type: "CONNECT"; target: Target; at: number }
  | { type: "PENDING"; timeoutS: number }
  | { type: "STREAMING" }
  | { type: "RECONNECTING" }
  | {
      type: "ERROR";
      reason: string;
      versions?: { phone: string; pc: string };
      versionSide?: "phone" | "pc";
    }
  | { type: "GO_HOME" }
  | { type: "ROUTE"; route: Route }
  | { type: "SHEET_OPEN"; sheet: Sheet }
  | { type: "SHEET_CLOSE" }
  | { type: "STATS"; stats?: Partial<StreamStats>; warnings?: Partial<StreamWarnings> }
  | { type: "CAMERA"; camera: CameraMode }
  | { type: "MIC" }
  | { type: "MIRROR" }
  | { type: "ROTATE" }
  | { type: "CAPS"; concurrent: boolean }
  | { type: "SETTINGS"; patch: Partial<Settings> }
  | { type: "KPC_ADD"; pc: KnownPc }
  | { type: "KPC_FORGET"; name: string };

export const initialState: AppState = {
  connection: { kind: "idle" },
  permissions: { camera: "unknown", mic: "unknown", notifications: "unknown" },
  permissionsDone: false,
  route: "home",
  sheet: null,
  knownPcs: [],
  settings: { background: true, keepScreenOn: false },
  stream: {
    camera: "back",
    mic: true,
    mirror: false,
    rotation: 0,
    caps: { concurrent: false },
    stats: { fps: null, bitrateKbps: null, rttMs: null, dropped: 0 },
    warnings: { slow: false, hot: false, battery: false },
  },
};

export function reducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "PERMISSION":
      return { ...state, permissions: { ...state.permissions, [action.kind]: action.status } };
    case "PERMISSIONS_DONE":
      return { ...state, permissionsDone: true };
    case "CONNECT":
      return {
        ...state,
        connection: { kind: "connecting", target: action.target, at: action.at },
        route: "home",
      };
    case "PENDING":
      return state.connection.kind === "connecting"
        ? {
            ...state,
            connection: {
              kind: "pending",
              target: state.connection.target,
              timeoutS: action.timeoutS,
            },
          }
        : state;
    case "STREAMING": {
      const conn = state.connection;
      if (conn.kind === "connecting" || conn.kind === "pending" || conn.kind === "reconnecting") {
        return { ...state, connection: { kind: "streaming", target: conn.target } };
      }
      return state;
    }
    case "RECONNECTING":
      return state.connection.kind === "streaming"
        ? { ...state, connection: { kind: "reconnecting", target: state.connection.target } }
        : state;
    case "ERROR": {
      const conn = state.connection;
      const target = conn.kind === "idle" ? null : conn.target;
      return {
        ...state,
        connection: {
          kind: "error",
          reason: action.reason,
          target,
          versions: action.versions,
          versionSide: action.versionSide,
        },
      };
    }
    case "GO_HOME":
      return { ...state, connection: { kind: "idle" }, route: "home" };
    case "ROUTE":
      return state.connection.kind === "idle" ? { ...state, route: action.route } : state;
    case "SHEET_OPEN":
      return { ...state, sheet: action.sheet };
    case "SHEET_CLOSE":
      return { ...state, sheet: null };
    case "STATS":
      return {
        ...state,
        stream: {
          ...state.stream,
          stats: { ...state.stream.stats, ...action.stats },
          warnings: { ...state.stream.warnings, ...action.warnings },
        },
      };
    case "CAMERA":
      return { ...state, stream: { ...state.stream, camera: action.camera } };
    case "MIC":
      return { ...state, stream: { ...state.stream, mic: !state.stream.mic } };
    case "MIRROR":
      return { ...state, stream: { ...state.stream, mirror: !state.stream.mirror } };
    case "ROTATE":
      return {
        ...state,
        stream: { ...state.stream, rotation: (state.stream.rotation + 90) % 360 },
      };
    case "CAPS":
      return {
        ...state,
        stream: { ...state.stream, caps: { ...state.stream.caps, concurrent: action.concurrent } },
      };
    case "SETTINGS":
      return { ...state, settings: { ...state.settings, ...action.patch } };
    case "KPC_ADD": {
      // O último adicionado fica por último ("último usado" na Home).
      const rest = state.knownPcs.filter((p) => p.name !== action.pc.name);
      return { ...state, knownPcs: [...rest, action.pc] };
    }
    case "KPC_FORGET":
      return { ...state, knownPcs: state.knownPcs.filter((p) => p.name !== action.name) };
  }
}

/* ----------------------- roteamento derivado ------------------------ */

export type ScreenId =
  | "permissions"
  | "home"
  | "scan"
  | "manual"
  | "knownPcs"
  | "connecting"
  | "awaiting"
  | "streaming"
  | "error";

/** Tela exibida derivada do estado (tabela ui/mobile.md §3.1). */
export function selectScreen(state: AppState): ScreenId {
  if (state.permissions.camera !== "granted" || !state.permissionsDone) return "permissions";
  switch (state.connection.kind) {
    case "idle":
      return state.route;
    case "connecting":
      return "connecting";
    case "pending":
      return "awaiting";
    case "streaming":
    case "reconnecting":
      return "streaming";
    case "error":
      return "error";
  }
}

/* --------------------- erros: razão → i18n -------------------------- */

export type ErrorCase = "badToken" | "notApproved" | "revoked" | "busy" | "version" | "generic";

/** Mapeia o código `reason` (WELCOME recusado / BYE) para o caso de erro da
 *  UI. É por CÓDIGO, nunca por texto da rede: qualquer razão desconhecida
 *  cai no `generic` (UT-2.09). */
export function reasonToI18nKey(reason: string): ErrorCase {
  switch (reason) {
    case "bad_token":
      return "badToken";
    case "not_approved":
      return "notApproved";
    case "revoked":
      return "revoked";
    case "busy":
      return "busy";
    case "version_mismatch":
      return "version";
    default:
      // timeout, protocol_error, bad_audio_ticket e qualquer string vinda
      // da rede que não seja um código conhecido.
      return "generic";
  }
}

/** Chave da explicação da tela de erro definitivo (§3.10). */
export function errorBodyKey(reason: string, versionSide: "phone" | "pc" = "phone"): string {
  switch (reason) {
    case "bad_token":
      return "common.errors.bad_token";
    case "not_approved":
      return "common.errors.not_approved";
    case "revoked":
      return "common.errors.revoked";
    case "busy":
      return "common.errors.busy";
    case "version_mismatch":
      return versionSide === "pc"
        ? "common.errors.version_mismatch.pc"
        : "common.errors.version_mismatch.phone";
    default:
      return "common.errors.generic";
  }
}

/* ------------------------- botão voltar ----------------------------- */

export type BackAction =
  | { kind: "closeSheet" }
  | { kind: "exitApp" }
  | { kind: "goHome" }
  | { kind: "cancelAndHome" }
  | { kind: "minimizeApp" }
  | { kind: "confirmStop" };

/** Comportamento do botão voltar por tela (tabela §3.1). */
export function backAction(state: AppState): BackAction {
  if (state.sheet) return { kind: "closeSheet" };
  switch (state.connection.kind) {
    case "idle": {
      const onPermissions = state.permissions.camera !== "granted" || !state.permissionsDone;
      if (onPermissions || state.route === "home") return { kind: "exitApp" };
      return { kind: "goHome" };
    }
    case "connecting":
    case "pending":
      return { kind: "cancelAndHome" };
    case "streaming":
    case "reconnecting":
      return state.settings.background ? { kind: "minimizeApp" } : { kind: "confirmStop" };
    case "error":
      return { kind: "goHome" };
  }
}

/* --------------------------- alvos (targets) ------------------------ */

/** Alvo a partir da URI bruta (QR/deep link). O parse serve só para o nome
 *  de exibição; a validação completa é o parser Kotlin (F2-T05a). */
export function targetFromRawUri(raw: string): Target {
  const res = parsePairingUri(raw);
  if (res.ok) {
    return {
      name: res.value.name || res.value.pc,
      host: res.value.hosts[0] ?? null,
      port: res.value.port,
      rawUri: raw,
      token: null,
    };
  }
  return { name: "", host: null, port: null, rawUri: raw, token: null };
}

/** Alvo digitado na tela Manual. */
export function manualTarget(host: string, port: number, token: string): Target {
  return { name: host, host, port, rawUri: null, token };
}
