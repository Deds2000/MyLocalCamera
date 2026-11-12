import { cn } from "@/lib/utils";

export type HealthState = "ok" | "warn" | "danger" | "muted" | "accent";

const DOT_COLOR: Record<HealthState, string> = {
  ok: "bg-ok",
  warn: "bg-warn",
  danger: "bg-danger",
  muted: "bg-text-muted",
  accent: "bg-brand",
};

export interface HealthIndicatorProps {
  /** Identificador do indicador (ex.: server, discovery, vcam, mic, adb). */
  kind: string;
  state: HealthState;
  /** Rótulo curto (ex.: ":48651", "Câmera pronta"). */
  detail: string;
  /** Clique abre a correção (comportamento do consumidor). */
  onClick?: () => void;
  className?: string;
}

/** Ponto + rótulo; estados ok/warn/danger/muted/accent (docs/ui/desktop.md §2.7). */
export function HealthIndicator({
  kind,
  state,
  detail,
  onClick,
  className,
}: HealthIndicatorProps) {
  const content = (
    <>
      <span
        aria-hidden="true"
        className={cn("h-2 w-2 shrink-0 rounded-full", DOT_COLOR[state])}
      />
      <span className="truncate text-xs text-text">{detail}</span>
    </>
  );

  const base = cn(
    "flex min-w-0 items-center gap-2 rounded-md px-1.5 py-1",
    className,
  );

  if (!onClick) {
    return <div data-kind={kind} className={base}>{content}</div>;
  }

  return (
    <button
      type="button"
      data-kind={kind}
      onClick={onClick}
      className={cn(
        base,
        "cursor-pointer text-left hover:bg-glass-strong",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        "focus-visible:ring-offset-2 focus-visible:ring-offset-background",
      )}
    >
      {content}
    </button>
  );
}
