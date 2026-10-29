// Deep link `localcamera://connect` (F2-T01): guarda a URI de pareamento
// recebida pelo Capacitor até a tela que a consome (F2-T03) lê-la.
//
// O `token` é segredo: nunca logar inteiro (máx. 4 chars) — ver AGENTS.md.

let pending: string | null = null;

/** Guarda a URI recebida e registra o log mascarado. */
export function setPendingDeepLink(url: string): void {
  pending = url;
  logDeepLink(url);
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
 * Loga só scheme, host, porta e os 4 primeiros chars do token —
 * nunca a URI completa nem o token inteiro.
 */
export function logDeepLink(url: string): void {
  try {
    const u = new URL(url);
    const token = u.searchParams.get("token");
    const port = u.searchParams.get("port");
    // Linha única: logcat não serializa objetos (apareceria [object Object]).
    console.info(
      `[deep-link] urlOpen scheme=${u.protocol.replace(":", "")} ` +
        `host=${u.hostname} port=${port ?? "-"} ` +
        `token=${token ? `${token.slice(0, 4)}…` : "-"}`,
    );
  } catch {
    // URI inválida: sem detalhes no log (poderia vazar parte do token).
    console.info("[deep-link] urlOpen (uri invalida, sem detalhes)");
  }
}
