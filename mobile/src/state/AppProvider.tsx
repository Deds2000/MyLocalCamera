// F2-T03 — Context do app: estado (reducer local) + serviços injetáveis.
//
// `qrScanner` é injetável para testes e para o scanner nativo futuro
// (F2-T07). `localCamera` (F2-T13) segue o mesmo padrão: quando não é
// injetada, cai em `getLocalCamera()`, que só devolve o plugin quando ele
// existe no app nativo. Com o plugin disponível, o provider consome os
// eventos reais e despacha as ações correspondentes; sem ele (web/testes)
// o reducer local segue sendo a única fonte — comportamento atual.
//
// `stats` chega do nativo já agregado (500 ms), mas o provider re-força o
// teto: evento com menos de 500 ms do último `STATS` despachado é
// descartado (CHEATSHEET: "Estatísticas para a UI — no máximo a cada
// 500 ms (2 Hz)").

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
  type Dispatch,
  type ReactNode,
} from "react";
import type { PluginListenerHandle } from "@capacitor/core";
import { parsePairingUri } from "@localcamera/shared/protocol";
import { clearPendingDeepLink, getPendingDeepLink, subscribeDeepLink } from "@/deep-link";
import {
  getLocalCamera,
  type ConnectOptions,
  type ConnectionChangedEvent,
  type ErrorEvent,
  type LocalCameraApi,
  type StoredKnownPc,
  type StatsEvent,
  type StateChangedEvent,
} from "@/plugins/LocalCamera";
import { getQrScanner, isPairingUri, type QrScannerApi } from "@/qr/qr-scanner";
import {
  initialState,
  reducer,
  errorReasonFromEventCode,
  targetFromRawUri,
  type AppAction,
  type AppState,
  type Connection,
  type KnownPc,
  type PermissionKind,
  type PermissionStatus,
  type Target,
} from "./connection";

const PERMISSION_KINDS: readonly PermissionKind[] = ["camera", "mic", "notifications"];

function appPermissionStatus(status: string | undefined): PermissionStatus {
  if (status === "granted" || status === "denied") return status;
  return "unknown";
}

export interface AppContextValue {
  state: AppState;
  dispatch: Dispatch<AppAction>;
  qrScanner: QrScannerApi;
  knownPcsLoading: boolean;
  requestPermission(kind: PermissionKind): Promise<void>;
  openPermissionSettings(): Promise<void>;
}

export interface AppProviderProps {
  initialState?: AppState;
  qrScanner?: QrScannerApi;
  /**
   * Plugin nativo (F2-T13). Sem injeção, usa `getLocalCamera()`
   * (`null` fora do app nativo) — mesmo padrão do `qrScanner`.
   */
  localCamera?: LocalCameraApi;
  children: ReactNode;
}

const AppContext = createContext<AppContextValue | null>(null);

/** Janela de agregação de `stats` — no máximo a cada 500 ms (2 Hz). */
const STATS_WINDOW_MS = 500;

/**
 * Target da conexão atual. A tela que inicia a conexão já despachou
 * `CONNECT` (target conhecido); o evento nativo `connecting` re-despacha o
 * mesmo target para manter a máquina sincronizada com o serviço.
 */
function currentTarget(conn: Connection): Target | null {
  return conn.kind === "idle" ? null : conn.target;
}

/**
 * Converte o alvo da UI nas opções do plugin nativo (F2-T13): URI do
 * QR/deep link (token só em memória) ou campos digitados na tela Manual.
 * PC conhecido usa somente seu `pcId`; host/token não entram no estado de
 * autenticação em JS.
 */
function connectOptions(target: Target): ConnectOptions | null {
  if (target.pcId) return { pcId: target.pcId };
  if (target.rawUri) {
    const parsed = parsePairingUri(target.rawUri);
    return parsed.ok
      ? { hosts: parsed.value.hosts, port: parsed.value.port, token: parsed.value.token }
      : null;
  }
  if (target.host && target.port && target.token) {
    return { hosts: [target.host], port: target.port, token: target.token };
  }
  return null;
}

function toAppKnownPc(pc: StoredKnownPc): KnownPc | null {
  const host = pc.hosts[0];
  if (!host || !Number.isInteger(pc.port) || pc.port < 1 || pc.port > 65535) return null;
  return {
    pcId: pc.pcId,
    name: pc.name,
    host,
    port: pc.port,
    lastSeenMs: pc.lastAccessed,
  };
}

export function AppProvider({
  initialState: initial,
  qrScanner,
  localCamera,
  children,
}: AppProviderProps) {
  const [state, dispatch] = useReducer(reducer, initial ?? initialState);
  const scanner = useMemo(() => qrScanner ?? getQrScanner(), [qrScanner]);
  const camera = useMemo(() => localCamera ?? getLocalCamera(), [localCamera]);
  const [knownPcsLoading, setKnownPcsLoading] = useState(camera !== null);

  const refreshPermissions = useCallback(async () => {
    if (!camera) return;
    try {
      const permissions = await camera.getPermissions();
      for (const kind of PERMISSION_KINDS) {
        dispatch({ type: "PERMISSION", kind, status: appPermissionStatus(permissions[kind]) });
      }
    } catch {
      // Uma falha de leitura nunca transforma uma permissão em concedida.
    }
  }, [camera, dispatch]);

  const requestPermission = useCallback(
    async (kind: PermissionKind) => {
      if (!camera) {
        // Web/jsdom mantém a simulação local existente; no Android, sempre
        // aguardamos a resposta real do plugin antes de liberar a tela.
        dispatch({ type: "PERMISSION", kind, status: "granted" });
        return;
      }
      try {
        const result = await camera.requestPermission({ kind });
        dispatch({ type: "PERMISSION", kind, status: appPermissionStatus(result.status) });
      } catch {
        await refreshPermissions();
      }
    },
    [camera, dispatch, refreshPermissions],
  );

  const openPermissionSettings = useCallback(async () => {
    if (!camera) return;
    try {
      await camera.openPermissionSettings();
    } catch {
      await refreshPermissions();
    }
  }, [camera, refreshPermissions]);

  useEffect(() => {
    if (!camera) return;
    void refreshPermissions();
    const onFocus = (): void => void refreshPermissions();
    const onVisibilityChange = (): void => {
      if (document.visibilityState === "visible") void refreshPermissions();
    };
    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => {
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [camera, refreshPermissions]);

  // Os handlers nativos leem o estado mais recente via ref (a assinatura
  // acontece uma única vez, quando o plugin está disponível).
  const stateRef = useRef(state);
  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  const knownPcsRequest = useRef(0);
  const refreshKnownPcs = useCallback(async () => {
    if (!camera) return;
    const request = ++knownPcsRequest.current;
    try {
      const { pcs } = await camera.getKnownPcs();
      if (request !== knownPcsRequest.current) return;
      dispatch({
        type: "KPC_REPLACE",
        pcs: (Array.isArray(pcs) ? pcs : []).map(toAppKnownPc).filter((pc): pc is KnownPc => pc !== null),
      });
    } catch {
      // Falha de leitura não apaga PCs já exibidos nem injeta texto nativo na UI.
    }
  }, [camera]);

  useEffect(() => {
    if (!camera) return;
    let active = true;
    void refreshKnownPcs().finally(() => {
      if (active) setKnownPcsLoading(false);
    });
    return () => {
      active = false;
    };
  }, [camera, refreshKnownPcs]);

  const dispatchAppAction = useCallback<Dispatch<AppAction>>(
    (action) => {
      if (action.type !== "KPC_FORGET" || !camera) {
        dispatch(action);
        return;
      }
      // Só remove da UI após a exclusão no PairingStore/Keystore.
      knownPcsRequest.current++;
      void camera.forgetPc({ pcId: action.pcId }).then(
        () => dispatch(action),
        () => void refreshKnownPcs(),
      );
    },
    [camera, refreshKnownPcs],
  );

  // F2-T13 (correção 2026-11-03): a tela/deep link só despacha CONNECT no
  // reducer local; sem este efeito o serviço nativo nunca era iniciado.
  // A chave evita repetir o connect quando o evento nativo `connecting`
  // re-despacha o mesmo alvo; um erro/retorno ao idle libera nova tentativa.
  const lastConnectKey = useRef<string | null>(null);
  useEffect(() => {
    if (!camera) return;
    const conn = state.connection;
    if (conn.kind === "idle" || conn.kind === "error") {
      lastConnectKey.current = null;
      return;
    }
    if (conn.kind !== "connecting") return;
    const key =
      (conn.target.pcId ? `pc:${conn.target.pcId}` : null) ??
      conn.target.rawUri ??
      `${conn.target.host ?? ""}|${conn.target.port ?? ""}|${conn.target.token ?? ""}`;
    if (lastConnectKey.current === key) return;
    const opts = connectOptions(conn.target);
    if (!opts) {
      lastConnectKey.current = key;
      dispatch({ type: "ERROR", reason: "invalid_args" });
      return;
    }
    lastConnectKey.current = key;
    void camera.connect(opts).catch(() => {
      lastConnectKey.current = null;
      dispatch({ type: "ERROR", reason: "invalid_args" });
    });
  }, [camera, dispatch, state.connection]);

  // Saída da transmissão (GO_HOME/Parar) encerra o serviço nativo; sem isso
  // a câmera continuaria aberta mesmo com a UI em idle.
  const nativeActive = useRef(false);
  useEffect(() => {
    if (!camera) return;
    const kind = state.connection.kind;
    const active =
      kind === "connecting" || kind === "pending" || kind === "streaming" || kind === "reconnecting";
    if (nativeActive.current && !active) void camera.disconnect().catch(() => {});
    nativeActive.current = active;
  }, [camera, state.connection.kind]);

  // Deep link `localcamera://connect` (F2-T01): consome a URI pendente
  // (cold start) e assina novas URIs (app já aberto) — antes só a leitura
  // inicial existia, então um link recebido com o app aberto era ignorado.
  useEffect(() => {
    const handle = (raw: string): void => {
      if (!isPairingUri(raw)) return;
      clearPendingDeepLink();
      dispatch({ type: "CONNECT", target: targetFromRawUri(raw), at: Date.now() });
    };
    const off = subscribeDeepLink(handle);
    const initial = getPendingDeepLink();
    if (initial) handle(initial);
    return off;
  }, [dispatch]);

  // F2-T13 — eventos do plugin nativo → ações do reducer local. Sem plugin
  // disponível (`camera === null`), nada muda: o reducer local segue
  // atendido apenas pelas telas (comportamento de web/testes atual).
  const lastStatsAt = useRef(Number.NEGATIVE_INFINITY);
  const lastStateJson = useRef("");

  useEffect(() => {
    if (!camera) return;

    const onConnectionChanged = (e: ConnectionChangedEvent): void => {
      switch (e.state) {
        case "connecting": {
          // A tela já despachou `CONNECT` (target conhecido); o evento
          // nativo confirma — re-despacha o mesmo target (sincroniza `at`).
          const target = currentTarget(stateRef.current.connection);
          if (target) dispatch({ type: "CONNECT", target, at: Date.now() });
          break;
        }
        case "pending":
          // `timeoutS` alimenta o countdown da tela de aprovação.
          dispatch({ type: "PENDING", timeoutS: e.timeoutS ?? 0 });
          break;
        case "streaming":
          dispatch({ type: "STREAMING" });
          void refreshKnownPcs();
          break;
        case "reconnecting":
          dispatch({ type: "RECONNECTING" });
          break;
        case "error":
          // Transição para o erro; o código real chega logo depois, no
          // evento `error{code}` (o plugin sempre emite os dois juntos).
          dispatch({ type: "ERROR", reason: "error" });
          break;
        case "idle":
          // No-op: o retorno ao idle é do usuário (botão "Parar" →
          // GO_HOME) — a UI não navega sozinha.
          break;
      }
    };

    const onStats = (e: StatsEvent): void => {
      // ≤ 2 Hz: descarta evento com menos de 500 ms do último emitido.
      const now = Date.now();
      if (now - lastStatsAt.current < STATS_WINDOW_MS) return;
      lastStatsAt.current = now;
      // `queueMs` não tem campo em `StreamStats` (F2-T03) — a forma do
      // estado não muda aqui (fora do escopo do cartão).
      dispatch({ type: "STATS", stats: { fps: e.fps, bitrateKbps: e.kbps, rttMs: e.rtt } });
    };

    const onError = (e: ErrorEvent): void => {
      // Só código (nunca texto da rede); a UI mapeia para i18n por código.
      dispatch({ type: "ERROR", reason: errorReasonFromEventCode(e.code) });
    };

    const onStateChanged = (e: StateChangedEvent): void => {
      // A UI usa o reducer local; guarda o último `STATE` (com `rev`) —
      // recuperação após recarregar o webview é via `getState()`.
      lastStateJson.current = e.state;
    };

    const handles: PluginListenerHandle[] = [];
    let disposed = false;
    void Promise.allSettled([
      camera.addListener("connectionChanged", onConnectionChanged),
      camera.addListener("stateChanged", onStateChanged),
      camera.addListener("stats", onStats),
      camera.addListener("error", onError),
    ])
      .then((results) => {
        const registered = results.flatMap((r) => (r.status === "fulfilled" ? [r.value] : []));
        if (disposed) {
          // Unmount antes do registro concluir: remove para não deixar
          // listener órfão no lado nativo.
          void Promise.allSettled(registered.map((h) => h.remove()));
          return;
        }
        handles.push(...registered);
      })
      .catch(() => {
        // `addListener` rejeita em plataforma sem o plugin (web): sem
        // plugin, não há evento — o reducer local segue valendo.
      });

    return () => {
      disposed = true;
      void Promise.allSettled(handles.map((h) => h.remove()));
    };
  }, [camera, refreshKnownPcs]);

  const value = useMemo(
    () => ({
      state,
      dispatch: dispatchAppAction,
      qrScanner: scanner,
      knownPcsLoading,
      requestPermission,
      openPermissionSettings,
    }),
    [state, dispatchAppAction, scanner, knownPcsLoading, requestPermission, openPermissionSettings],
  );

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

export function useApp(): AppContextValue {
  const ctx = useContext(AppContext);
  if (!ctx) throw new Error("useApp deve ser usado dentro de <AppProvider>");
  return ctx;
}
