import { useCallback, useEffect, useRef, useState } from "react";
import {
  INITIAL_STATS,
  type Mode,
  type UiStats,
  autoConfig,
  run,
  stopFeed,
} from "./engine";

export default function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [stats, setStats] = useState<UiStats>(INITIAL_STATS);
  const startedRef = useRef(false);

  const start = useCallback(
    (mode: Mode, seconds: number, jsonFinish = false) => {
      if (!canvasRef.current) return;
      if (stats.running) stopFeed();
      void run({
        mode,
        seconds,
        canvas: canvasRef.current,
        onTick: setStats,
        onDone: setStats,
        jsonFinish,
      });
    },
    [stats.running],
  );

  useEffect(() => {
    // Automação para medição: `--auto A|B --seconds N [--json <arq>]`.
    // Com `--json`, o app grava o resumo e encerra sozinho ao final.
    if (startedRef.current) return;
    startedRef.current = true;
    void (async () => {
      const cfg = await autoConfig();
      if (cfg) start(cfg.mode, cfg.seconds, cfg.json !== null);
    })();
  }, [start]);

  const s = stats;
  return (
    <div className="wrap">
      <canvas ref={canvasRef} width={960} height={540} className="video" />
      <div className="bar">
        <button
          onClick={() => start("A", 60)}
          disabled={s.running}
          title="WebCodecs: AU H.264 via Channel"
        >
          A · WebCodecs
        </button>
        <button
          onClick={() => start("B", 60)}
          disabled={s.running}
          title="JPEG 960x540 via Channel"
        >
          B · JPEG
        </button>
        <button onClick={() => void stopFeed()} disabled={!s.running}>
          Parar
        </button>
        <span className="stat">modo: {s.mode ?? "—"}</span>
        <span className="stat">
          suportado(A): {s.supported === null ? "—" : s.supported ? "sim" : "não"}
        </span>
        <span className="stat">fps: {s.fps.toFixed(1)}</span>
        <span className="stat">descartes: {s.drops}</span>
        <span className="stat">MB/s: {s.mbps.toFixed(2)}</span>
        <span className="stat">lat: {s.latencyMs.toFixed(0)} ms</span>
        <span className="stat">mem: {s.memMb.toFixed(0)} MB</span>
        <span className="stat">js: {s.jsHeapMb.toFixed(0)} MB</span>
        <span className="stat">t: {s.elapsedS}s</span>
      </div>
      {s.error && <div className="err">{s.error}</div>}
    </div>
  );
}
