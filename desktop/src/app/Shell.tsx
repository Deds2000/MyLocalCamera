import { useEffect, useState } from "react";
import Titlebar from "./Titlebar";
import { panelWidth, previewPadding } from "./layout";

/** Grade do app (F3-T01): titlebar 40 px · (preview + painel) · rodapé 32 px. */

function useWindowWidth(): number {
  const [width, setWidth] = useState<number>(() =>
    typeof window === "undefined" ? 1280 : window.innerWidth,
  );
  useEffect(() => {
    const onResize = () => setWidth(window.innerWidth);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);
  return width;
}

export default function Shell() {
  const width = useWindowWidth();

  return (
    <div className="flex h-full flex-col bg-bg text-text">
      <Titlebar />
      <div className="flex min-h-0 flex-1">
        <section
          data-testid="preview"
          className="flex min-w-0 flex-1 items-center justify-center bg-raised"
          style={{ padding: previewPadding(width) }}
        >
          <div
            data-testid="preview-frame"
            className="aspect-video max-h-full max-w-full rounded-lg border border-glass-border bg-glass"
          />
        </section>
        <aside
          data-testid="panel"
          className="h-full shrink-0 border-l border-glass-border bg-raised"
          style={{ width: panelWidth(width) }}
        />
      </div>
      <footer
        data-testid="footer"
        className="h-8 shrink-0 border-t border-glass-border bg-raised"
      />
    </div>
  );
}
