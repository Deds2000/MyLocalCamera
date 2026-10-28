// Motor de medição do spike S5.
// Wire format (Channel<Vec<u8>> do Rust):
//   byte 0  : flags (bit 0 = keyframe)
//   bytes 1-8: u64 LE, unix epoch em milissegundos (momento do envio no Rust)
//   resto   : payload (AU H.264 Annex-B no modo A; JPEG no modo B)
import { Channel, invoke } from "@tauri-apps/api/core";
import { buildSummary, type RunSummary } from "./summary";

export type Mode = "A" | "B";

export interface UiStats {
  mode: Mode | null;
  running: boolean;
  supported: boolean | null; // resultado de isConfigSupported (modo A)
  fps: number;
  drops: number; // descartes acumulados
  mbps: number; // MB/s do Channel (janela de 10 s)
  latencyMs: number; // média da janela (envio→desenho)
  memMb: number; // working set do processo (Rust)
  jsHeapMb: number; // heap JS (sanity)
  elapsedS: number;
  error: string | null;
}

export const INITIAL_STATS: UiStats = {
  mode: null,
  running: false,
  supported: null,
  fps: 0,
  drops: 0,
  mbps: 0,
  latencyMs: 0,
  memMb: 0,
  jsHeapMb: 0,
  elapsedS: 0,
  error: null,
};

const CSV_HEADER = "mode,t_s,fps,drops,mbps,lat_ms,mem_mb,js_heap_mb";
const MAX_QUEUE_A = 6; // AUs pendentes de decode antes de descartar
const MAX_PENDING_B = 8; // JPEGs em createImageBitmap antes de descartar

class Metrics {
  draws = 0;
  bytes = 0;
  drops = 0;
  latSamples: number[] = []; // amostras envio→desenho (p50/p95 no resumo final)
  private pDraws = 0;
  private pBytes = 0;
  private pDrops = 0;
  private pLatSum = 0;
  private pLatN = 0;
  private latSum = 0;
  private latN = 0;

  reset() {
    this.draws = 0;
    this.bytes = 0;
    this.drops = 0;
    this.latSamples = [];
    this.pDraws = 0;
    this.pBytes = 0;
    this.pDrops = 0;
    this.pLatSum = 0;
    this.pLatN = 0;
    this.latSum = 0;
    this.latN = 0;
  }

  recordDraw(latencyMs: number) {
    this.draws += 1;
    if (latencyMs >= 0) {
      this.latSum += latencyMs;
      this.latN += 1;
      this.latSamples.push(latencyMs);
    }
  }
  recordBytes(n: number) {
    this.bytes += n;
  }
  recordDrop() {
    this.drops += 1;
  }

  /** Estatísticas da janela dos últimos `w` segundos (reseta a janela). */
  window(w: number) {
    const nLat = this.latN - this.pLatN;
    const out = {
      fps: (this.draws - this.pDraws) / w,
      drops: this.drops - this.pDrops,
      mbps: (this.bytes - this.pBytes) / w / 1048576,
      latencyMs: nLat > 0 ? (this.latSum - this.pLatSum) / nLat : 0,
    };
    this.pDraws = this.draws;
    this.pBytes = this.bytes;
    this.pDrops = this.drops;
    this.pLatSum = this.latSum;
    this.pLatN = this.latN;
    return out;
  }
}

/** Converte o payload do Channel em Uint8Array, aguentando ArrayBuffer,
 * views, arrays JSON ou base64 (o formato exato varia entre versões do Tauri). */
function toU8(raw: unknown): Uint8Array | null {
  if (raw instanceof ArrayBuffer) return new Uint8Array(raw);
  if (raw instanceof Uint8Array) return raw;
  if (raw && typeof raw === "object") {
    const v = raw as {
      buffer?: unknown;
      byteOffset?: number;
      length?: number;
    };
    if (v.buffer instanceof ArrayBuffer) {
      return new Uint8Array(v.buffer, v.byteOffset ?? 0, v.length ?? undefined);
    }
  }
  if (Array.isArray(raw)) {
    const n = raw as number[];
    if (n.every((x) => Number.isInteger(x) && x >= 0 && x <= 255)) {
      return Uint8Array.from(n);
    }
    return null;
  }
  if (typeof raw === "string") {
    const bin = atob(raw);
    const u = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i);
    return u;
  }
  return null;
}

/** Falha silenciosa: o CSV é auxiliar; nunca pode bloquear o resumo JSON. */
async function csvAppend(line: string) {
  try {
    await invoke("csv_append", { line, header: CSV_HEADER });
  } catch {
    /* cenário: CWD errado/sem pasta spikes/s5 — o JSON segue */
  }
}

function jsHeapMb(): number {
  const m = (performance as unknown as { memory?: { usedJSHeapSize: number } })
    .memory;
  return m ? m.usedJSHeapSize / 1024 : 0;
}

async function processMemMb(): Promise<number> {
  try {
    return (await invoke<{ mem_mb: number }>("process_info")).mem_mb;
  } catch {
    return 0;
  }
}

async function processInfo(): Promise<{ memMb: number; pid: number }> {
  try {
    const r = await invoke<{ mem_mb: number; pid: number }>("process_info");
    return { memMb: r.mem_mb, pid: r.pid };
  } catch {
    return { memMb: 0, pid: 0 };
  }
}

/** Grava o resumo final e encerra o app (modo auto com `--json`). */
async function autoFinish(sum: RunSummary): Promise<void> {
  try {
    await invoke("auto_finish", { summary: JSON.stringify(sum) });
  } catch {
    /* no Rust há uma trava de segurança que encerra o app mesmo assim */
  }
}

interface RunOpts {
  mode: Mode;
  seconds: number;
  canvas: HTMLCanvasElement;
  onTick: (s: UiStats) => void;
  onDone: (s: UiStats) => void;
  /** Quando verdadeiro, ao fim chama `auto_finish` (grava JSON + encerra o app). */
  jsonFinish?: boolean;
}

export async function run(o: RunOpts): Promise<void> {
  const m = new Metrics();
  m.reset();
  const { memMb: memStart, pid } = await processInfo();
  let t0 = 0; // preenchido após o feed iniciar (duração real da medição)
  const latQueue: number[] = []; // sendTs dos AUs enviados (modo A)
  let decoder: VideoDecoder | null = null;
  let stopped = false;
  let pendingB = 0;
  let frameIdx = 0;
  const ctx2d = o.canvas.getContext("2d")!;
  ctx2d.clearRect(0, 0, o.canvas.width, o.canvas.height);

  const draw = (latencyMs: number) => m.recordDraw(latencyMs);

  const onMessage = (raw: unknown) => {
    if (stopped) return;
    const u8 = toU8(raw);
    if (!u8 || u8.length < 9) return;
    const flags = u8[0] ?? 0;
    // u64 LE no offset 1 (não alinhado → DataView)
    const sendTs = Number(new DataView(u8.buffer, u8.byteOffset + 1, 8).getBigUint64(0, true));
    m.recordBytes(u8.length - 9);

    if (o.mode === "A") {
      if (!decoder || decoder.state !== "configured") return;
      if (decoder.decodeQueueSize > MAX_QUEUE_A) {
        m.recordDrop();
        return;
      }
      decoder.decode(
        new EncodedVideoChunk({
          type: flags & 1 ? "key" : "delta",
          timestamp: frameIdx * (1000 / 30),
          data: u8.slice(9),
        }),
      );
      latQueue.push(sendTs);
      frameIdx += 1;
    } else {
      if (pendingB >= MAX_PENDING_B) {
        m.recordDrop();
        return;
      }
      pendingB += 1;
      const blob = new Blob([u8.slice(9)], { type: "image/jpeg" });
      createImageBitmap(blob)
        .then((bmp) => {
          pendingB -= 1;
          if (stopped) {
            bmp.close();
            return;
          }
          ctx2d.drawImage(bmp, 0, 0);
          bmp.close();
          draw(Date.now() - sendTs);
        })
        .catch(() => {
          pendingB -= 1;
          m.recordDrop();
        });
    }
  };

  const chan = new Channel<Uint8Array>(onMessage);

  try {
    if (o.mode === "A") {
      const cfg = {
        codec: "avc1.640028",
        avc: { format: "annexb" } as const,
        hardwareAcceleration: "prefer-hardware" as const,
        optimizeForLatency: true,
      };
      const { supported } = await VideoDecoder.isConfigSupported(cfg);
      if (!supported) {
        if (o.jsonFinish) {
          await autoFinish(
            buildSummary({
              mode: o.mode,
              seconds: o.seconds,
              elapsedS: 0,
              draws: 0,
              bytes: 0,
              drops: 0,
              latencySamples: [],
              supported: false,
              memStartMb: memStart,
              memEndMb: memStart,
              pid,
              error: "isConfigSupported = false",
            }),
          );
        }
        o.onDone(
          stats(false, { supported: false, error: "isConfigSupported = false" }),
        );
        return;
      }
      decoder = new VideoDecoder({
        output: (f) => {
          if (stopped) {
            f.close();
            return;
          }
          ctx2d.drawImage(f, 0, 0);
          const ts = latQueue.shift();
          draw(ts !== undefined ? Date.now() - ts : -1);
          f.close();
        },
        error: (e) => {
          console.error("VideoDecoder error", e);
        },
      });
      decoder.configure(cfg);
    }

    await invoke(`start_mode_${o.mode.toLowerCase()}`, { channel: chan });
    t0 = performance.now();
    let lastTick = t0;

    const tickTimer = window.setInterval(async () => {
      const now = performance.now();
      const w = (now - lastTick) / 1000;
      lastTick = now;
      const ws = m.window(w);
      const memMb = await processMemMb();
      const s = stats(true, {
        fps: ws.fps,
        drops: m.drops,
        mbps: ws.mbps,
        latencyMs: ws.latencyMs,
        memMb,
        jsHeapMb: jsHeapMb(),
        elapsedS: Math.round((now - t0) / 1000),
      });
      // Em auto+json o resumo sai pelo JSON (e o run-s5.ps1 grava o CSV) —
      // não poluir o medicoes.csv rastreado.
      if (!o.jsonFinish) {
        await csvAppend(csvLine(o.mode, String(Math.round((now - t0) / 1000)), ws, memMb));
      }
      o.onTick(s);
    }, 10000);

    window.setTimeout(async () => {
      stopped = true;
      window.clearInterval(tickTimer);
      const now = performance.now();
      const w = Math.max((now - lastTick) / 1000, 1);
      const ws = m.window(w);
      const memMb = await processMemMb();
      await invoke("stop_feed").catch(() => {});
      if (decoder) {
        try {
          await decoder.flush();
        } catch {
          /* pode já estar fechado */
        }
        decoder.close();
        decoder = null;
      }
      if (!o.jsonFinish) {
        await csvAppend(csvLine(o.mode, "FIM", ws, memMb));
      }
      if (o.jsonFinish) {
        await autoFinish(
          buildSummary({
            mode: o.mode,
            seconds: o.seconds,
            elapsedS: (now - t0) / 1000,
            draws: m.draws,
            bytes: m.bytes,
            drops: m.drops,
            latencySamples: m.latSamples,
            supported: o.mode === "A" ? true : null,
            memStartMb: memStart,
            memEndMb: memMb,
            pid,
          }),
        );
      }
      o.onDone(
        stats(false, {
          supported: o.mode === "A" ? true : null,
          fps: ws.fps,
          drops: m.drops,
          mbps: ws.mbps,
          latencyMs: ws.latencyMs,
          memMb,
          jsHeapMb: jsHeapMb(),
          elapsedS: o.seconds,
        }),
      );
    }, o.seconds * 1000);

    o.onTick(stats(true, { supported: o.mode === "A" ? true : null }));
  } catch (e) {
    stopped = true;
    if (o.jsonFinish) {
      // Mesmo em falha: grava o JSON (com error) e encerra, p/ o script não travar.
      await autoFinish(
        buildSummary({
          mode: o.mode,
          seconds: o.seconds,
          elapsedS: t0 > 0 ? (performance.now() - t0) / 1000 : 0,
          draws: m.draws,
          bytes: m.bytes,
          drops: m.drops,
          latencySamples: m.latSamples,
          supported: null,
          memStartMb: memStart,
          memEndMb: memStart,
          pid,
          error: String(e),
        }),
      );
    }
    o.onDone(stats(false, { error: String(e) }));
  }

  function stats(running: boolean, extra: Partial<UiStats>): UiStats {
    return {
      ...INITIAL_STATS,
      mode: o.mode,
      running,
      supported: null,
      ...extra,
    };
  }
}

function csvLine(
  mode: Mode,
  t: string,
  ws: { fps: number; drops: number; mbps: number; latencyMs: number },
  memMb: number,
): string {
  return [
    mode,
    t,
    ws.fps.toFixed(2),
    String(ws.drops),
    ws.mbps.toFixed(3),
    ws.latencyMs.toFixed(0),
    String(memMb),
    jsHeapMb().toFixed(1),
  ].join(",");
}

/** Cancela o feed no Rust (botão Parar). */
export async function stopFeed() {
  await invoke("stop_feed").catch(() => {});
}

export async function autoConfig(): Promise<
  { mode: Mode; seconds: number; json: string | null } | null
> {
  const r = (await invoke<
    { mode: string | null; seconds: number; json: string | null } | null
  >("auto_config")) as { mode: string | null; seconds: number; json: string | null } | null;
  if (!r || r.mode === null) return null;
  const mode = r.mode as Mode;
  if (mode !== "A" && mode !== "B") return null;
  return { mode, seconds: r.seconds, json: r.json };
}
