import { useStore } from "zustand";
import { PreviewFrame } from "@/components/PreviewFrame";
import { appStore } from "@/store";
import { EmptyState } from "./EmptyState";
import { previewPadding } from "@/app/layout";

interface PreviewAreaProps {
  width: number;
}

export function PreviewArea({ width }: PreviewAreaProps) {
  const state = useStore(appStore, (store) => store.connection.state);

  return (
    <section
      data-testid="preview"
      className="flex min-h-0 min-w-0 flex-1 items-center justify-center bg-raised"
      style={{ padding: previewPadding(width) }}
    >
      <PreviewFrame className="max-h-full max-w-full">
        {state === "waiting" ? (
          <div className="h-full w-full overflow-hidden p-3">
            <EmptyState />
          </div>
        ) : (
          <div data-testid="preview-placeholder" className="h-full w-full bg-raised" />
        )}
      </PreviewFrame>
    </section>
  );
}
