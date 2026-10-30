import * as React from "react";

import { cn } from "@/lib/utils";

export interface BigToggleProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  /** Ícone (ReactNode). */
  icon: React.ReactNode;
  /** Rótulo do controle. */
  label: string;
  /** Estado pressionado. */
  pressed: boolean;
}

export const BigToggle = React.forwardRef<HTMLButtonElement, BigToggleProps>(
  ({ icon, label, pressed, className, ...props }, ref) => (
    <button
      ref={ref}
      type="button"
      aria-pressed={pressed}
      className={cn(
        "flex h-14 w-full items-center justify-center gap-3 rounded-md border px-4 text-base font-medium",
        "transition-colors motion-reduce:transition-none",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:pointer-events-none disabled:opacity-45",
        pressed
          ? "bg-primary text-primary-foreground border-primary"
          : "bg-glass border-glass-border text-text hover:bg-glass-strong",
        className,
      )}
      {...props}
    >
      <span aria-hidden="true" className="shrink-0">
        {icon}
      </span>
      <span>{label}</span>
    </button>
  ),
);
BigToggle.displayName = "BigToggle";