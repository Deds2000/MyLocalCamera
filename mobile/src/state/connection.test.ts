// F2-T03 — UT-2.08 (selectScreen), UT-2.09 (reasonToI18nKey), UT-2.12 (backAction).

import { describe, expect, it } from "vitest";
import {
  backAction,
  errorBodyKey,
  initialState,
  reasonToI18nKey,
  selectScreen,
  type AppState,
  type Connection,
  type ErrorCase,
  type Target,
} from "./connection";

const TARGET: Target = {
  name: "DESKTOP-X",
  host: "192.168.0.10",
  port: 48650,
  rawUri: null,
  token: null,
};

const allGranted = { camera: "granted", mic: "granted", notifications: "granted" } as const;

function state(connection: Connection, patch: Partial<AppState> = {}): AppState {
  return {
    ...initialState,
    permissions: { ...allGranted },
    permissionsDone: true,
    connection,
    ...patch,
  };
}

describe("selectScreen — tabela §3.1 (UT-2.08)", () => {
  it("permissões obrigatórias pendentes → Permissões (qualquer estado de conexão)", () => {
    // Câmera desconhecida (primeira execução).
    expect(selectScreen(initialState)).toBe("permissions");
    // Câmera negada permanentemente.
    expect(
      selectScreen({
        ...initialState,
        permissions: { camera: "denied", mic: "granted", notifications: "granted" },
        permissionsDone: true,
      }),
    ).toBe("permissions");
    // Câmera concedida, mas o fluxo de permissões não foi concluído.
    expect(
      selectScreen({ ...initialState, permissions: { ...allGranted }, permissionsDone: false }),
    ).toBe("permissions");
  });

  it("Idle → Home/Scan/Manual/KnownPcs conforme a rota local", () => {
    expect(selectScreen(state({ kind: "idle" }))).toBe("home");
    expect(selectScreen(state({ kind: "idle" }, { route: "scan" }))).toBe("scan");
    expect(selectScreen(state({ kind: "idle" }, { route: "manual" }))).toBe("manual");
    expect(selectScreen(state({ kind: "idle" }, { route: "knownPcs" }))).toBe("knownPcs");
  });

  it("Connecting/Pending/Streaming/Reconnecting/Erro → tela correspondente", () => {
    expect(selectScreen(state({ kind: "connecting", target: TARGET, at: 0 }))).toBe("connecting");
    expect(selectScreen(state({ kind: "pending", target: TARGET, timeoutS: 30 }))).toBe("awaiting");
    expect(selectScreen(state({ kind: "streaming", target: TARGET }))).toBe("streaming");
    // Reconectando é sobreposição na Transmitindo (§3.8) — mesma tela.
    expect(selectScreen(state({ kind: "reconnecting", target: TARGET }))).toBe("streaming");
    expect(selectScreen(state({ kind: "error", reason: "revoked", target: TARGET }))).toBe("error");
  });
});

describe("reasonToI18nKey — mapeamento por código (UT-2.09)", () => {
  const expected: Record<string, ErrorCase> = {
    bad_token: "badToken",
    not_approved: "notApproved",
    revoked: "revoked",
    busy: "busy",
    version_mismatch: "version",
    timeout: "generic",
    protocol_error: "generic",
    bad_audio_ticket: "generic",
  };

  it.each(Object.entries(expected))("%s → %s", (reason, want) => {
    expect(reasonToI18nKey(reason)).toBe(want);
  });

  it("razão desconhecida (qualquer texto da rede) → generic, sem o texto vazar", () => {
    const raw = "texto-arbitrario-da-rede";
    const key = reasonToI18nKey(raw);
    expect(key).toBe("generic");
    expect(key).not.toContain(raw);
  });

  it("a chave de corpo também é por código (versões por lado)", () => {
    expect(errorBodyKey("bad_token")).toBe("common.errors.bad_token");
    expect(errorBodyKey("version_mismatch", "pc")).toBe("common.errors.version_mismatch.pc");
    expect(errorBodyKey("version_mismatch", "phone")).toBe("common.errors.version_mismatch.phone");
    expect(errorBodyKey("nada-conhecido")).toBe("common.errors.generic");
  });
});

describe("backAction — botão voltar por tela (UT-2.12)", () => {
  it("sheet aberto → fecha o sheet (em qualquer tela)", () => {
    expect(backAction(state({ kind: "idle" }, { sheet: "settings" }))).toEqual({
      kind: "closeSheet",
    });
    expect(backAction(state({ kind: "streaming", target: TARGET }, { sheet: "settings" }))).toEqual(
      { kind: "closeSheet" },
    );
  });

  it("Home → sai do app (idem tela de Permissões)", () => {
    expect(backAction(state({ kind: "idle" }))).toEqual({ kind: "exitApp" });
    expect(backAction(initialState)).toEqual({ kind: "exitApp" });
    expect(
      backAction({
        ...initialState,
        permissions: { ...allGranted },
        permissionsDone: false,
      }),
    ).toEqual({ kind: "exitApp" });
  });

  it("Scan/Manual/KnownPcs → volta à Home", () => {
    expect(backAction(state({ kind: "idle" }, { route: "scan" }))).toEqual({ kind: "goHome" });
    expect(backAction(state({ kind: "idle" }, { route: "manual" }))).toEqual({ kind: "goHome" });
    expect(backAction(state({ kind: "idle" }, { route: "knownPcs" }))).toEqual({ kind: "goHome" });
  });

  it("Conectando/Aguardando → cancela a tentativa e volta à Home", () => {
    expect(backAction(state({ kind: "connecting", target: TARGET, at: 0 }))).toEqual({
      kind: "cancelAndHome",
    });
    expect(backAction(state({ kind: "pending", target: TARGET, timeoutS: 30 }))).toEqual({
      kind: "cancelAndHome",
    });
  });

  it("Transmitindo: 'tela apagada' ligado → minimiza; desligado → confirma parar", () => {
    expect(backAction(state({ kind: "streaming", target: TARGET }))).toEqual({
      kind: "minimizeApp",
    }); // padrão settings.background = true
    expect(
      backAction(
        state(
          { kind: "streaming", target: TARGET },
          {
            settings: { background: false, keepScreenOn: false },
          },
        ),
      ),
    ).toEqual({ kind: "confirmStop" });
    expect(
      backAction(
        state(
          { kind: "reconnecting", target: TARGET },
          {
            settings: { background: false, keepScreenOn: false },
          },
        ),
      ),
    ).toEqual({ kind: "confirmStop" });
  });

  it("Erro definitivo → volta à Home", () => {
    expect(backAction(state({ kind: "error", reason: "busy", target: TARGET }))).toEqual({
      kind: "goHome",
    });
  });
});
