import * as React from "react";

import { cn } from "../lib/cn";

export interface EmptyStateProps {
  icon: React.ReactNode;
  title: string;
  description?: string;
  actions?: React.ReactNode;
  className?: string;
}

export function EmptyState({
  icon,
  title,
  description,
  actions,
  className,
}: EmptyStateProps) {
  return (
    <div
      data-testid="empty-state"
      className={cn(
        "flex flex-col items-center justify-center gap-2 px-6 py-10 text-center",
        className,
      )}
    >
      <span aria-hidden="true" className="mb-1 text-text-muted">
        {icon}
      </span>
      <span className="text-sm font-medium text-text">{title}</span>
      {description ? (
        <span className="max-w-sm text-xs text-text-muted">{description}</span>
      ) : null}
      {actions ? (
        <div className="mt-2 flex flex-wrap items-center justify-center gap-2">
          {actions}
        </div>
      ) : null}
    </div>
  );
}
