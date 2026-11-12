import { cn } from "../lib/cn";

export interface LevelMeterProps {
  /** Nível atual, 0 a 1. */
  level: number;
  /** Pico (hold), 0 a 1. */
  peak: number;
  className?: string;
}

function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value));
}

/**
 * Medidor de nível desenhado com CSS/transform (sem layout): `scaleX` no
 * preenchimento e `translateX` no pico — atualizações ≤ 20 Hz.
 * Cinza quando desligado (level = 0).
 */
export function LevelMeter({ level, peak, className }: LevelMeterProps) {
  const lvl = clamp01(level);
  const pk = clamp01(peak);
  const active = lvl > 0;
  return (
    <div
      data-testid="level-meter"
      className={cn(
        "relative h-2 w-full overflow-hidden rounded-full bg-glass-strong",
        className,
      )}
    >
      <div
        data-testid="level-fill"
        aria-hidden="true"
        className={cn(
          "h-full w-full origin-left will-change-transform",
          active ? "bg-ok" : "bg-text-muted",
        )}
        style={{ transform: `scaleX(${active ? lvl : 0})` }}
      />
      <div
        data-testid="level-peak"
        aria-hidden="true"
        className="pointer-events-none absolute inset-y-0 left-0 w-full will-change-transform"
        style={{ transform: `translateX(${pk * 100}%)` }}
      >
        <span
          className={cn(
            "absolute inset-y-0 left-0 w-px",
            active ? "bg-warn" : "bg-text-muted",
          )}
        />
      </div>
    </div>
  );
}
