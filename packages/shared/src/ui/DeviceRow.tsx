import * as React from "react";

import { cn } from "../lib/cn";

export interface DeviceRowProps {
  icon: React.ReactNode;
  title: string;
  subtitle?: string;
  badge?: React.ReactNode;
  action?: React.ReactNode;
  className?: string;
}

export function DeviceRow({
  icon,
  title,
  subtitle,
  badge,
  action,
  className,
}: DeviceRowProps) {
  return (
    <div
      data-testid="device-row"
      className={cn(
        "flex items-center gap-3 rounded-md border border-glass-border bg-glass px-3 py-2",
        className,
      )}
    >
      <span
        aria-hidden="true"
        className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-glass-strong text-text-muted"
      >
        {icon}
      </span>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-sm font-medium text-text">{title}</span>
        {subtitle ? (
          <span className="truncate text-xs text-text-muted">{subtitle}</span>
        ) : null}
      </span>
      {badge ? <span className="shrink-0">{badge}</span> : null}
      {action ? <span className="shrink-0">{action}</span> : null}
    </div>
  );
}
