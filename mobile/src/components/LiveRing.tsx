import { cn } from "@/lib/utils";

export interface LiveRingProps {
  /** Transmissão ativa. */
  active?: boolean;
  /** Diâmetro em px. Padrão 12. */
  size?: number;
  className?: string;
  /** Rótulo acessível. */
  label?: string;
}

export function LiveRing({ active = true, size = 12, className, label }: LiveRingProps) {
  const reduced =
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const animate = active && !reduced;

  return (
    <span
      role="status"
      aria-label={label}
      className={cn("relative inline-flex items-center justify-center", className)}
      style={{ width: size, height: size }}
    >
      <span
        aria-hidden="true"
        className={cn(
          "absolute inset-0 rounded-full bg-ok",
          animate && "animate-pulse-ring motion-reduce:animate-none",
        )}
      />
      <span
        aria-hidden="true"
        className="relative rounded-full bg-ok"
        style={{ width: size * 0.4, height: size * 0.4 }}
      />
    </span>
  );
}