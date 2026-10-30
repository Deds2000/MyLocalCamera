import * as React from "react";
import {
  Wifi,
  WifiOff,
  Loader2,
  ShieldAlert,
  RefreshCw,
  TriangleAlert,
} from "lucide-react";

import { cn } from "@/lib/utils";

export type ConnectionState =
  | "idle"
  | "connecting"
  | "pending"
  | "streaming"
  | "reconnecting"
  | "error";

interface StateStyle {
  color: string;
  Icon: React.ComponentType<{ className?: string }>;
  spin: boolean;
}

const STATE_STYLE: Record<ConnectionState, StateStyle> = {
  idle: { color: "text-text-muted", Icon: WifiOff, spin: false },
  connecting: { color: "text-brand", Icon: Loader2, spin: true },
  pending: { color: "text-warn", Icon: ShieldAlert, spin: false },
  streaming: { color: "text-ok", Icon: Wifi, spin: false },
  reconnecting: { color: "text-warn", Icon: RefreshCw, spin: true },
  error: { color: "text-danger", Icon: TriangleAlert, spin: false },
};

export interface StatusChipProps {
  state: ConnectionState;
  label: string;
  icon?: React.ReactNode;
  className?: string;
}

export function StatusChip({ state, label, icon, className }: StatusChipProps) {
  const { color, Icon, spin } = STATE_STYLE[state];
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border border-glass-border bg-glass-strong px-2.5 py-1 text-xs font-medium",
        color,
        className,
      )}
    >
      <span
        aria-hidden="true"
        className={cn("shrink-0", spin && "animate-spin motion-reduce:animate-none")}
      >
        {icon ?? <Icon className="h-3.5 w-3.5" />}
      </span>
      <span>{label}</span>
    </span>
  );
}