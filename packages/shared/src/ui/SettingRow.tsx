import * as React from "react";
import { Smartphone } from "lucide-react";

import { cn } from "../lib/cn";

export type AppliesTo = "pc" | "phone" | "both";

export interface SettingRowProps {
  label: string;
  description?: string;
  control: React.ReactNode;
  /** Onde a config tem efeito; `phone`/`both` mostram o ícone sutil (reinicia a captura no celular). */
  appliesTo?: AppliesTo;
  className?: string;
}

export function SettingRow({
  label,
  description,
  control,
  appliesTo,
  className,
}: SettingRowProps) {
  const showsPhoneIcon = appliesTo === "phone" || appliesTo === "both";
  return (
    <div
      data-testid="setting-row"
      className={cn("flex items-center justify-between gap-3", className)}
    >
      <div className="flex min-w-0 flex-col gap-0.5">
        <span className="flex items-center gap-1.5 text-sm text-text">
          {label}
          {showsPhoneIcon ? (
            <Smartphone
              aria-hidden="true"
              className="h-3.5 w-3.5 shrink-0 text-text-muted"
            />
          ) : null}
        </span>
        {description ? (
          <span className="text-xs text-text-muted">{description}</span>
        ) : null}
      </div>
      <div className="shrink-0">{control}</div>
    </div>
  );
}
