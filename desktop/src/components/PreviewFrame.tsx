import * as React from "react";

import { cn } from "@/lib/utils";

export interface PreviewFrameProps {
  children?: React.ReactNode;
  overlay?: React.ReactNode;
  className?: string;
}

/**
 * Moldura 16:9 do preview (ui-spec): raio `xl`, `border-glass-border`,
 * `shadow-glass`, letterbox `bg-raised`. Overlays com fundo sólido `bg/60` —
 * nunca `backdrop-filter`/blur sobre o vídeo.
 */
export function PreviewFrame({ children, overlay, className }: PreviewFrameProps) {
  return (
    <div
      data-testid="preview-frame"
      className={cn(
        "relative flex aspect-video w-full items-center justify-center overflow-hidden",
        "rounded-xl border border-glass-border bg-raised shadow-glass",
        className,
      )}
    >
      <div className="relative h-full w-full">{children}</div>
      {overlay ? (
        <div className="absolute inset-0 flex items-center justify-center bg-black/60">
          {overlay}
        </div>
      ) : null}
    </div>
  );
}
