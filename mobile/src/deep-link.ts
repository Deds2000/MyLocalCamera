// Deep link `localcamera://connect` (F2-T01): guarda a URI de pareamento
// recebida pelo Capacitor até a tela que a consome (F2-T03) lê-la.
//
// O `token` é segredo: nunca logar inteiro (máx. 4 chars) — ver AGENTS.md.

let pending: string | null = null;

type DeepLinkListener = (url: string) => void;
const listeners = new Set<DeepLinkListener>();

/** Assina novas URIs (app já aberto). Devolve o cancelamento. */
export function subscribeDeepLink(listener: DeepLinkListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Guarda a URI recebida e registra o log mascarado. */
export function setPendingDeepLink(url: string): void {
  pending = url;
  logDeepLink(url);
  // App já aberto: o `AppProvider` precisa reagir sem remontar.
  for (const listener of listeners) listener(url);
}

/** URI pendente (ou `null` se nenhuma). A tela de destino deve chamá-lo após consumir. */
export function getPendingDeepLink(): string | null {
  return pending;
}

/** Limpa a URI pendente (chamar após consumir ou em erro). */
export function clearPendingDeepLink(): void {
  pending = null;
}

/**
 * Loga só scheme, host e porta — nem mesmo um prefixo do token.
 */
export function logDeepLink(url: string): void {
  try {
    const u = new URL(url);
    const port = u.searchParams.get("port");
    console.info(
      `[deep-link] urlOpen scheme=${u.protocol.replace(":", "")} ` +
        `host=${u.hostname} port=${port ?? "-"}`,
    );
  } catch {
    // URI inválida: sem detalhes no log (poderia vazar parte do token).
    console.info("[deep-link] urlOpen (uri invalida, sem detalhes)");
  }
}
