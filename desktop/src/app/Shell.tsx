import { useEffect, useState } from "react";
import Titlebar from "./Titlebar";
import { panelWidth } from "./layout";
import { ConnectionTab } from "@/features/connection/ConnectionTab";
import { PreviewArea } from "@/features/preview/PreviewArea";

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
        <PreviewArea width={width} />
        <aside
          data-testid="panel"
          className="h-full min-w-0 shrink-0 border-l border-glass-border bg-raised"
          style={{ width: panelWidth(width) }}
        >
          <ConnectionTab />
        </aside>
      </div>
      <footer
        data-testid="footer"
        className="h-8 shrink-0 border-t border-glass-border bg-raised"
      />
    </div>
  );
}
