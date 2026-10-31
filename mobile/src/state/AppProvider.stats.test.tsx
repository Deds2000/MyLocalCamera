// F2-T13 — UT: agregação de eventos `stats` no `AppProvider` (≤ 2 Hz).
//
// Injeta um plugin fake via a prop `localCamera` e emite `stats` em > 2 Hz:
// no máximo 1 dispatch de `STATS` por janela de 500 ms — eventos que
// chegam antes disso são descartados (CHEATSHEET: "Estatísticas para a UI
// — no máximo a cada 500 ms (2 Hz)").

import { act, waitFor } from "@testing-library/react";
import { useEffect } from "react";
import { renderWithProviders } from "@localcamera/shared/test";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  type ConnectionChangedEvent,
  type ErrorEvent,
  type LocalCameraApi,
  type StatsEvent,
  type StateChangedEvent,
} from "@/plugins/LocalCamera";
import { AppProvider, useApp } from "@/state/AppProvider";
import { initialState } from "@/state/connection";

type EventName = "connectionChanged" | "stateChanged" | "stats" | "error";
type AnyEvent = ConnectionChangedEvent | StateChangedEvent | StatsEvent | ErrorEvent;

/** Plugin fake: captura os listeners registrados por evento (injeção via
 *  a prop `localCamera` do `AppProvider`). */
function makeFakePlugin(): {
  api: LocalCameraApi;
  emit: (name: EventName, e: AnyEvent) => void;
  listenerCount: (name: EventName) => number;
} {
  const buckets: Record<EventName, Set<(e: AnyEvent) => void>> = {
    connectionChanged: new Set(),
    stateChanged: new Set(),
    stats: new Set(),
    error: new Set(),
  };

  const api = {
    getCapabilities: async (): Promise<{ caps: unknown }> => ({ caps: {} }),
    connect: async (): Promise<void> => {},
    disconnect: async (): Promise<void> => {},
    getState: async (): Promise<{ state: string }> => ({ state: "" }),
    addListener: async (name: EventName, fn: (e: AnyEvent) => void) => {
      buckets[name].add(fn);
      return {
        remove: async () => {
          buckets[name].delete(fn);
        },
      };
    },
  } as LocalCameraApi;

  return {
    api,
    emit: (name, e) => {
      for (const fn of buckets[name]) fn(e);
    },
    listenerCount: (name) => buckets[name].size,
  };
}

/** Observa `state.stream.stats.fps` e registra cada mudança (uma entrada
 *  por dispatch de `STATS` que muda o estado). */
function StatsProbe({ log }: { log: number[] }) {
  const { state } = useApp();
  const fps = state.stream.stats.fps ?? 0;
  useEffect(() => {
    if (log[log.length - 1] !== fps) log.push(fps);
  });
  return null;
}

describe("AppProvider — agregação de stats (F2-T13, ≤ 2 Hz)", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("emite no máximo 1 STATS por 500 ms, descartando o resto (> 2 Hz)", async () => {
    // Só o relógio `Date` fica fake: o init do i18n segue usando timers
    // reais (mesma razão do teste "Aguardando" em screens.test.tsx).
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(1_000_000);

    const fake = makeFakePlugin();
    const log: number[] = [];

    await act(async () => {
      await renderWithProviders(
        <AppProvider initialState={initialState} localCamera={fake.api}>
          <StatsProbe log={log} />
        </AppProvider>,
      );
    });
    // A assinatura assíncrona (addListener) deve estar concluída.
    await waitFor(() => expect(fake.listenerCount("stats")).toBe(1));

    // 4 eventos em 300 ms (> 2 Hz): só o primeiro é despachado.
    await act(async () => {
      fake.emit("stats", { fps: 10, kbps: 4000, queueMs: 12, rtt: 5 });
      vi.setSystemTime(1_000_100);
      fake.emit("stats", { fps: 11, kbps: 4100, queueMs: 13, rtt: 6 });
      vi.setSystemTime(1_000_200);
      fake.emit("stats", { fps: 12, kbps: 4200, queueMs: 14, rtt: 7 });
      vi.setSystemTime(1_000_300);
      fake.emit("stats", { fps: 13, kbps: 4300, queueMs: 15, rtt: 8 });
    });
    expect(log).toEqual([0, 10]);

    // A exatamente 500 ms a janela reabre (fronteira: 500 não é < 500).
    await act(async () => {
      vi.setSystemTime(1_000_500);
      fake.emit("stats", { fps: 20, kbps: 5000, queueMs: 20, rtt: 10 });
    });
    expect(log).toEqual([0, 10, 20]);
  });
});
