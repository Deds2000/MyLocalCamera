// F2-T03 — Helper de rótulo "visto há…" para PCs conhecidos.
// O texto vai para a chave `mobile.home.seen` / último uso ({{when}}).

import { useState } from "react";

/** `Date.now()` capturado na montagem da tela — evita chamada impura no
 *  render (o rótulo se renova a cada remontagem, ex.: ao voltar à Home). */
export function useNowMs(): number {
  const [now] = useState(() => Date.now());
  return now;
}

/** Rótulo relativo de tempo visto (ex.: "há 2 horas" / "2 hours ago"). */
export function formatSeen(lastSeenMs: number | null, now: number, lng?: string): string {
  if (lastSeenMs === null) return "";
  const locale = (lng ?? "").toLowerCase().startsWith("pt") ? "pt-BR" : "en";
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  const diffMin = Math.round((lastSeenMs - now) / 60_000);
  if (diffMin > -1) return rtf.format(0, "minute");
  if (diffMin > -60) return rtf.format(diffMin, "minute");
  if (diffMin > -1440) return rtf.format(Math.round(diffMin / 60), "hour");
  return rtf.format(Math.round(diffMin / 1440), "day");
}
