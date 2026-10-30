// F2-T03 — Context do app: estado (reducer local) + serviços injetáveis.
//
// `qrScanner` é injetável para testes e para o scanner nativo futuro
// (F2-T07/F2-T13). Stats/avisos chegam pelo dispatch (ação `STATS`) —
// em produção, o plugin Kotlin (F2-T13) despacha a ≤ 2 Hz.

import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useReducer,
  type Dispatch,
  type ReactNode,
} from "react";
import { clearPendingDeepLink, getPendingDeepLink } from "@/deep-link";
import { getQrScanner, isPairingUri, type QrScannerApi } from "@/qr/qr-scanner";
import {
  initialState,
  reducer,
  targetFromRawUri,
  type AppAction,
  type AppState,
} from "./connection";

export interface AppContextValue {
  state: AppState;
  dispatch: Dispatch<AppAction>;
  qrScanner: QrScannerApi;
}

export interface AppProviderProps {
  initialState?: AppState;
  qrScanner?: QrScannerApi;
  children: ReactNode;
}

const AppContext = createContext<AppContextValue | null>(null);

export function AppProvider({ initialState: initial, qrScanner, children }: AppProviderProps) {
  const [state, dispatch] = useReducer(reducer, initial ?? initialState);
  const scanner = useMemo(() => qrScanner ?? getQrScanner(), [qrScanner]);

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

  const value = useMemo(() => ({ state, dispatch, qrScanner: scanner }), [state, scanner]);

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

export function useApp(): AppContextValue {
  const ctx = useContext(AppContext);
  if (!ctx) throw new Error("useApp deve ser usado dentro de <AppProvider>");
  return ctx;
}
