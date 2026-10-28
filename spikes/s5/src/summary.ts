// Resumo final da execução do spike S5, escrito no JSON de `--json`
// pelo comando `auto_finish` (Rust) ao fim da medição.

export interface LatencySummary {
  avg: number;
  p50: number;
  p95: number;
}

export interface RunSummary {
  mode: "A" | "B";
  seconds: number;
  supported: boolean | null; // isConfigSupported (só modo A)
  fps: number; // quadros desenhados / duração total
  drops: number; // descartes totais
  mbps: number; // MB/s médios do Channel (duração total)
  latency_ms: LatencySummary;
  mem_mb: { start: number; end: number };
  pid: number;
  error: string | null;
}

export interface SummaryInput {
  mode: "A" | "B";
  seconds: number;
  elapsedS: number;
  draws: number;
  bytes: number;
  drops: number;
  latencySamples: number[];
  supported: boolean | null;
  memStartMb: number;
  memEndMb: number;
  pid: number;
  error?: string | null;
}

/** Percentil por rank mais próximo (dados já podem chegar não ordenados). */
export function percentile(samples: number[], p: number): number {
  if (samples.length === 0) return 0;
  const s = [...samples].sort((a, b) => a - b);
  const rank = Math.min(s.length - 1, Math.max(0, Math.ceil(p * s.length) - 1));
  return s[rank] ?? 0;
}

export function latencyStats(samples: number[]): LatencySummary {
  const n = samples.length;
  const avg = n > 0 ? samples.reduce((a, b) => a + b, 0) / n : 0;
  return { avg, p50: percentile(samples, 0.5), p95: percentile(samples, 0.95) };
}

export function buildSummary(i: SummaryInput): RunSummary {
  const t = Math.max(i.elapsedS, 1);
  return {
    mode: i.mode,
    seconds: i.seconds,
    supported: i.supported,
    fps: i.draws / t,
    drops: i.drops,
    mbps: i.bytes / t / 1048576,
    latency_ms: latencyStats(i.latencySamples),
    mem_mb: { start: i.memStartMb, end: i.memEndMb },
    pid: i.pid,
    error: i.error ?? null,
  };
}
