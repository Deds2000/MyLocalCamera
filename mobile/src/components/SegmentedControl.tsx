import * as React from "react";

import { cn } from "@/lib/utils";

export interface SegmentedOption {
  value: string;
  label: string;
  disabled?: boolean;
  disabledReason?: string;
}

export interface SegmentedControlProps {
  options: SegmentedOption[];
  value: string;
  onChange: (value: string) => void;
  disabledReasons?: Record<string, string>;
  className?: string;
  ariaLabel?: string;
}

function isDisabled(
  opt: SegmentedOption,
  disabledReasons?: Record<string, string>,
): boolean {
  return Boolean(opt.disabled || disabledReasons?.[opt.value]);
}

function reasonFor(
  opt: SegmentedOption,
  disabledReasons?: Record<string, string>,
): string | undefined {
  return opt.disabledReason ?? disabledReasons?.[opt.value];
}

export function SegmentedControl({
  options,
  value,
  onChange,
  disabledReasons,
  className,
  ariaLabel,
}: SegmentedControlProps) {
  const initialIndex = React.useMemo(() => {
    const sel = options.findIndex(
      (o) => o.value === value && !isDisabled(o, disabledReasons),
    );
    if (sel >= 0) return sel;
    const firstEnabled = options.findIndex(
      (o) => !isDisabled(o, disabledReasons),
    );
    return firstEnabled >= 0 ? firstEnabled : 0;
  }, [options, value, disabledReasons]);

  const [focusIndex, setFocusIndex] = React.useState(initialIndex);

  const nextEnabled = React.useCallback(
    (start: number, dir: 1 | -1): number => {
      let idx = start;
      for (let i = 0; i < options.length; i++) {
        idx = (idx + dir + options.length) % options.length;
        const opt = options[idx];
        if (opt && !isDisabled(opt, disabledReasons)) return idx;
      }
      return start;
    },
    [options, disabledReasons],
  );

  const handleKeyDown = (
    e: React.KeyboardEvent,
    idx: number,
  ): void => {
    if (e.key === "ArrowRight" || e.key === "ArrowDown") {
      e.preventDefault();
      const next = nextEnabled(idx, 1);
      setFocusIndex(next);
      document.getElementById(`seg-${next}`)?.focus();
    } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
      e.preventDefault();
      const prev = nextEnabled(idx, -1);
      setFocusIndex(prev);
      document.getElementById(`seg-${prev}`)?.focus();
    } else if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      const opt = options[idx];
      if (opt && !isDisabled(opt, disabledReasons)) onChange(opt.value);
    }
  };

  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      className={cn("inline-flex rounded-md bg-muted p-1", className)}
    >
      {options.map((opt, idx) => {
        const disabled = isDisabled(opt, disabledReasons);
        const reason = reasonFor(opt, disabledReasons);
        const selected = opt.value === value;
        return (
          <button
            key={opt.value}
            id={`seg-${idx}`}
            type="button"
            role="radio"
            aria-checked={selected}
            aria-disabled={disabled || undefined}
            tabIndex={idx === focusIndex ? 0 : -1}
            disabled={disabled}
            title={disabled ? reason : undefined}
            onClick={() => {
              if (!disabled) onChange(opt.value);
            }}
            onKeyDown={(e) => handleKeyDown(e, idx)}
            className={cn(
              "min-h-[44px] rounded-sm px-3 py-1 text-sm font-medium transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              "motion-reduce:transition-none",
              selected
                ? "bg-background text-foreground shadow"
                : "text-muted-foreground hover:text-foreground",
              disabled && "opacity-45 cursor-not-allowed",
            )}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}