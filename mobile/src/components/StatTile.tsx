import { cn } from "@/lib/utils";

export interface StatTileProps {
  label: string;
  value: string | number;
  unit?: string;
  hint?: string;
  className?: string;
}

export function StatTile({ label, value, unit, hint, className }: StatTileProps) {
  return (
    <div
      className={cn(
        "flex flex-col gap-1 rounded-md border border-glass-border bg-glass-strong px-3 py-2",
        className,
      )}
      role="group"
      aria-label={label}
    >
      <span className="text-xs text-text-muted">{label}</span>
      <div className="flex items-baseline gap-1">
        <span className="font-mono text-sm tabular-nums text-text">{value}</span>
        {unit ? <span className="text-xs text-text-muted">{unit}</span> : null}
      </div>
      {hint ? <span className="text-xs text-text-muted">{hint}</span> : null}
    </div>
  );
}