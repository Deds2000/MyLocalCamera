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
  useContext,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  type Dispatch,
  type ReactNode,
} from "react";
import type { PluginListenerHandle } from "@capacitor/core";
import { clearPendingDeepLink, getPendingDeepLink } from "@/deep-link";
import {
  getLocalCamera,
  type ConnectionChangedEvent,
  type ErrorEvent,
  type LocalCameraApi,
  type StatsEvent,
  type StateChangedEvent,
} from "@/plugins/LocalCamera";
import { getQrScanner, isPairingUri, type QrScannerApi } from "@/qr/qr-scanner";
import {
  initialState,
  reducer,
  targetFromRawUri,
  type AppAction,
  type AppState,
  type Connection,
  type Target,
} from "./connection";

export interface AppContextValue {
  state: AppState;
  dispatch: Dispatch<AppAction>;
  qrScanner: QrScannerApi;
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

export function AppProvider({
  initialState: initial,
  qrScanner,
  localCamera,
  children,
}: AppProviderProps) {
  const [state, dispatch] = useReducer(reducer, initial ?? initialState);
  const scanner = useMemo(() => qrScanner ?? getQrScanner(), [qrScanner]);
  const camera = useMemo(() => localCamera ?? getLocalCamera(), [localCamera]);

  // Os handlers nativos leem o estado mais recente via ref (a assinatura
  // acontece uma única vez, quando o plugin está disponível).
  const stateRef = useRef(state);
  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  // Deep link `localcamera://connect` (F2-T01): se chegar uma URI pendente
  // (checagem simples scheme/host), usa o alvo e vai a "Conectando".
  useEffect(() => {
    const raw = getPendingDeepLink();
    if (!raw) return;
    if (isPairingUri(raw)) {
      dispatch({ type: "CONNECT", target: targetFromRawUri(raw), at: Date.now() });
    }
    clearPendingDeepLink();
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
      dispatch({ type: "ERROR", reason: e.code });
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
  }, [camera]);

  const value = useMemo(() => ({ state, dispatch, qrScanner: scanner }), [state, scanner]);

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

export function useApp(): AppContextValue {
  const ctx = useContext(AppContext);
  if (!ctx) throw new Error("useApp deve ser usado dentro de <AppProvider>");
  return ctx;
}
