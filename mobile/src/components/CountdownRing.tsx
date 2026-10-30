import { cn } from "@/lib/utils";

export interface CountdownRingProps {
  /** Valor atual (segundos restantes). */
  seconds: number;
  /** Valor total (segundos). */
  total: number;
  /** Diâmetro em px. Padrão 64. */
  size?: number;
  /** Espessura do traço. Padrão 4. */
  strokeWidth?: number;
  className?: string;
  /** Rótulo acessível. */
  ariaLabel?: string;
}

export function CountdownRing({
  seconds,
  total,
  size = 64,
  strokeWidth = 4,
  className,
  ariaLabel,
}: CountdownRingProps) {
  const reduced =
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const safeTotal = total > 0 ? total : 1;
  const clamped = Math.min(Math.max(seconds, 0), safeTotal);
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const progress = clamped / safeTotal;
  const offset = circumference * (1 - progress);

  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      role="progressbar"
      aria-valuenow={Math.round(clamped)}
      aria-valuemin={0}
      aria-valuemax={Math.round(safeTotal)}
      aria-label={ariaLabel}
      className={cn("block", className)}
    >
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        fill="none"
        stroke="hsl(var(--muted))"
        strokeWidth={strokeWidth}
        opacity={0.35}
      />
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        fill="none"
        stroke="hsl(var(--primary))"
        strokeWidth={strokeWidth}
        strokeLinecap="round"
        strokeDasharray={circumference}
        strokeDashoffset={offset}
        transform={`rotate(-90 ${size / 2} ${size / 2})`}
        style={{
          transition: reduced
            ? "none"
            : "stroke-dashoffset 200ms cubic-bezier(0.22,1,0.36,1)",
        }}
      />
    </svg>
  );
}