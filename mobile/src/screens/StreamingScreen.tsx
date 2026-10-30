// F2-T03 — Tela: Transmitindo (a principal) — ui/mobile.md §3.8.
//
// Sem prévia de câmera nesta versão. Stats/avisos chegam pelo provider
// (ação `STATS`, ≤ 2 Hz). `reduce-glass` ligado nesta tela (§3.2). A
// sobreposição de reconexão por mDNS (Fase 5) fica fora; aqui é o painel
// mínimo de etapas (§3.8).

import { useTranslation } from "react-i18next";
import {
  Circle,
  FlipHorizontal2,
  Loader2,
  Mic,
  RotateCw,
  Square,
  TriangleAlert,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { LiveRing } from "@/components/LiveRing";
import { SegmentedControl } from "@/components/SegmentedControl";
import { StatTile } from "@/components/StatTile";
import { StatusChip } from "@/components/StatusChip";
import { useApp } from "@/state/AppProvider";
import type { CameraMode } from "@/state/connection";
import { cn } from "@/lib/utils";

function ReconnectingOverlay() {
  const { t } = useTranslation();
  const { dispatch } = useApp();
  const scan = () => {
    dispatch({ type: "GO_HOME" });
    dispatch({ type: "ROUTE", route: "scan" });
  };
  return (
    <div className="absolute inset-0 z-10 flex items-center justify-center bg-bg/85 p-5">
      <div
        role="dialog"
        aria-label={t("mobile.reconnect.title")}
        className="w-full max-w-sm rounded-lg border border-glass-border bg-bg-raised p-5 shadow-overlay"
      >
        <h2 className="text-lg font-semibold">{t("mobile.reconnect.title")}</h2>
        <ul className="mt-4 flex flex-col gap-3 text-sm">
          <li className="flex items-center gap-2">
            <Loader2
              className="h-4 w-4 animate-spin text-warn motion-reduce:animate-none"
              aria-hidden="true"
            />
            <span>{t("mobile.reconnect.discovery")}</span>
          </li>
          <li className="flex items-center gap-2 text-text-muted">
            <Circle className="h-4 w-4" aria-hidden="true" />
            <span>{t("mobile.reconnect.lastAddresses")}</span>
          </li>
          <li className="flex items-center gap-2 text-text-muted">
            <Circle className="h-4 w-4" aria-hidden="true" />
            <span>{t("mobile.reconnect.failed")}</span>
          </li>
        </ul>
        <div className="mt-5 flex flex-col gap-2">
          <Button className="h-12 w-full" onClick={scan}>
            {t("mobile.scan.title")}
          </Button>
          <Button
            variant="outline"
            className="h-12 w-full"
            onClick={() => dispatch({ type: "GO_HOME" })}
          >
            {t("common.actions.cancel")}
          </Button>
        </div>
      </div>
    </div>
  );
}

function WarnBanner({ children }: { children: string }) {
  return (
    <div className="flex items-center gap-2 rounded-md border border-warn/40 bg-warn/10 px-3 py-2 text-sm text-warn">
      <TriangleAlert className="h-4 w-4 shrink-0" aria-hidden="true" />
      <span>{children}</span>
    </div>
  );
}

export function StreamingScreen() {
  const { t } = useTranslation();
  const { state, dispatch } = useApp();

  const conn = state.connection;
  if (conn.kind !== "streaming" && conn.kind !== "reconnecting") return null;

  const reconnecting = conn.kind === "reconnecting";
  const { stats, warnings, camera, mic, mirror, rotation, caps } = state.stream;

  const cameraLabel =
    camera === "dual"
      ? t("mobile.stream.dualBoth")
      : t(camera === "back" ? "common.camera.back" : "common.camera.front");
  const bitrate = stats.bitrateKbps !== null ? (stats.bitrateKbps / 1000).toFixed(1) : "";

  const cameraOptions = [
    { value: "back", label: t("common.camera.back") },
    { value: "front", label: t("common.camera.front") },
    {
      value: "dual",
      label: t("common.camera.dual"),
      disabled: !caps.concurrent,
      disabledReason: t("mobile.stream.dualUnavailable"),
    },
  ];

  return (
    <main className="relative flex h-full flex-col gap-4 overflow-y-auto px-5 pb-6 pt-8 reduce-glass">
      <header className="flex items-center justify-between gap-3">
        <StatusChip
          state={reconnecting ? "reconnecting" : "streaming"}
          label={t(reconnecting ? "common.state.reconnecting" : "common.state.streaming")}
        />
        <span className="min-w-0 truncate text-sm font-medium">{conn.target.name}</span>
      </header>

      <div className="flex flex-col items-center gap-2 py-4">
        <div className="flex items-center gap-3">
          <LiveRing active={!reconnecting} size={18} label={t("common.state.live")} />
          <span className="text-sm font-semibold tracking-wide">{t("common.state.live")}</span>
        </div>
        <p className="text-sm text-text-muted">
          {t("mobile.stream.status", { name: conn.target.name })}
        </p>
      </div>

      {(warnings.slow || warnings.hot || warnings.battery) && (
        <div className="flex flex-col gap-2">
          {warnings.slow && <WarnBanner>{t("mobile.stream.warnSlow")}</WarnBanner>}
          {warnings.hot && <WarnBanner>{t("mobile.stream.warnHot")}</WarnBanner>}
          {warnings.battery && <WarnBanner>{t("mobile.stream.warnBattery")}</WarnBanner>}
        </div>
      )}

      <div className="grid grid-cols-2 gap-2">
        <StatTile label={t("mobile.stream.bitrate")} value={bitrate} unit="Mbps" />
        <StatTile label={t("mobile.stream.latency")} value={stats.rttMs ?? ""} unit="ms" />
        <StatTile label={t("mobile.stream.dropped")} value={stats.dropped} />
        <StatTile label={t("mobile.stream.camera")} value={cameraLabel} />
      </div>

      <SegmentedControl
        options={cameraOptions}
        value={camera}
        onChange={(value) => dispatch({ type: "CAMERA", camera: value as CameraMode })}
        ariaLabel={t("mobile.stream.camera")}
        className="w-full"
      />

      <div className="grid grid-cols-3 gap-2">
        <Button
          variant={mic ? "default" : "secondary"}
          className="h-14 w-full"
          aria-pressed={mic}
          onClick={() => dispatch({ type: "MIC" })}
        >
          <Mic aria-hidden="true" />
          <span className="truncate">{t("mobile.stream.mic")}</span>
        </Button>
        <Button
          variant={mirror ? "default" : "secondary"}
          className="h-14 w-full"
          aria-pressed={mirror}
          onClick={() => dispatch({ type: "MIRROR" })}
        >
          <FlipHorizontal2 aria-hidden="true" />
          <span className="truncate">{t("mobile.stream.mirror")}</span>
        </Button>
        <Button
          variant="secondary"
          className="h-14 w-full"
          onClick={() => dispatch({ type: "ROTATE" })}
        >
          <RotateCw aria-hidden="true" style={{ transform: `rotate(${rotation}deg)` }} />
          <span className="truncate">{t("mobile.stream.rotate")}</span>
        </Button>
      </div>

      <div className={cn("mt-auto pt-2")}>
        <Button
          variant="destructive"
          className="h-14 w-full"
          onClick={() => dispatch({ type: "GO_HOME" })}
        >
          <Square aria-hidden="true" />
          {t("mobile.stream.stop")}
        </Button>
      </div>

      {reconnecting && <ReconnectingOverlay />}
    </main>
  );
}
