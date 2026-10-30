// F2-T03c — Catálogo de componentes (só dev, rota `?catalog`).
// Mostra os componentes próprios em todos os estados para revisão visual
// (UT-2.13) e acessibilidade automática (UT-2.14). ui/mobile.md §3.14.
// Os rótulos vêm das chaves `catalog.*` (i18n) e reutilizam as chaves
// existentes de estados/câmeras/estatísticas do app.

import { useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Power } from "lucide-react";
import {
  BigToggle,
  CountdownRing,
  LiveRing,
  SegmentedControl,
  StatTile,
  StatusChip,
} from "@/components/catalog-exports";
import type { ConnectionState } from "@/components/catalog-exports";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

const CHIP_STATES: ConnectionState[] = [
  "idle",
  "connecting",
  "pending",
  "streaming",
  "reconnecting",
  "error",
];

const RING_TOTAL = 30;
const RING_VALUES = [30, 15, 5];

function Section({
  id,
  title,
  children,
}: {
  id: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <section aria-labelledby={id} className="flex flex-col gap-3 py-5 first:pt-1 last:pb-1">
      <h2 id={id} className="text-base font-semibold">
        {title}
      </h2>
      {children}
    </section>
  );
}

export function CatalogScreen() {
  const { t } = useTranslation();
  const [camera, setCamera] = useState("back");

  return (
    <main className="flex h-full flex-col overflow-y-auto px-5 pb-8 pt-8">
      <header className="pb-2">
        <h1 className="text-xl font-semibold">{t("catalog.title")}</h1>
        <p className="mt-1 text-sm text-text-muted">{t("catalog.subtitle")}</p>
      </header>

      <div className="flex flex-col divide-y divide-glass-border">
        <Section id="catalog-statuschip" title={t("catalog.statusChip")}>
          <div className="flex flex-wrap gap-2">
            {CHIP_STATES.map((s) => (
              <span key={s} data-testid={`catalog-statuschip-${s}`}>
                <StatusChip state={s} label={t(`common.state.${s}`)} />
              </span>
            ))}
          </div>
        </Section>

        <Section id="catalog-countdown" title={t("catalog.countdownRing")}>
          <div className="flex items-start gap-6">
            {RING_VALUES.map((s) => (
              <div
                key={s}
                data-testid={`catalog-ring-${s}`}
                className="flex flex-col items-center gap-1.5"
              >
                <CountdownRing
                  seconds={s}
                  total={RING_TOTAL}
                  ariaLabel={t("catalog.countdownSeconds", { seconds: s })}
                />
                <span className="text-xs text-text-muted">
                  {t("catalog.countdownSeconds", { seconds: s })}
                </span>
              </div>
            ))}
          </div>
        </Section>

        <Section id="catalog-bigtoggle" title={t("catalog.bigToggle")}>
          <div className="flex flex-col gap-3">
            <BigToggle
              data-testid="catalog-bigtoggle-on"
              icon={<Power aria-hidden="true" />}
              label={t("catalog.toggleOn")}
              pressed
            />
            <BigToggle
              data-testid="catalog-bigtoggle-off"
              icon={<Power aria-hidden="true" />}
              label={t("catalog.toggleOff")}
              pressed={false}
            />
          </div>
        </Section>

        <Section id="catalog-liverring" title={t("catalog.liveRing")}>
          <div className="flex items-center gap-8">
            <div
              data-testid="catalog-liverring-active"
              className="flex flex-col items-center gap-2"
            >
              <LiveRing active label={t("catalog.liveOn")} />
              <span className="text-xs text-text-muted">{t("catalog.liveOn")}</span>
            </div>
            <div
              data-testid="catalog-liverring-paused"
              className="flex flex-col items-center gap-2"
            >
              <LiveRing active={false} label={t("catalog.liveOff")} />
              <span className="text-xs text-text-muted">{t("catalog.liveOff")}</span>
            </div>
          </div>
        </Section>

        <Section id="catalog-stattile" title={t("catalog.statTile")}>
          <p className="text-xs text-text-muted">{t("catalog.statNormal")}</p>
          <div data-testid="catalog-stattile-normal" className="grid grid-cols-3 gap-2">
            <StatTile
              label={t("mobile.stream.bitrate")}
              value={t("common.units.mbps", { value: 6.2 })}
            />
            <StatTile
              label={t("mobile.stream.latency")}
              value={t("common.units.ms", { value: 24 })}
            />
            <StatTile label={t("mobile.stream.dropped")} value={0} />
          </div>
          <p className="text-xs text-text-muted">{t("catalog.statSlow")}</p>
          <div data-testid="catalog-stattile-slow" className="grid gap-2">
            <StatTile
              label={t("mobile.stream.bitrate")}
              value={t("common.units.mbps", { value: 0.9 })}
              hint={t("mobile.stream.warnSlow")}
            />
          </div>
        </Section>

        <Section id="catalog-segmented" title={t("catalog.segmentedControl")}>
          <div data-testid="catalog-segmented">
            <SegmentedControl
              options={[
                { value: "back", label: t("common.camera.back") },
                { value: "front", label: t("common.camera.front") },
                {
                  value: "dual",
                  label: t("common.camera.dual"),
                  disabled: true,
                  disabledReason: t("mobile.stream.dualUnavailable"),
                },
              ]}
              value={camera}
              onChange={setCamera}
              ariaLabel={t("catalog.segmentedControl")}
            />
          </div>
        </Section>

        <Section id="catalog-dialog" title={t("catalog.dialog")}>
          <Dialog>
            <DialogTrigger asChild>
              <Button variant="outline" data-testid="catalog-dialog-open">
                {t("catalog.dialogOpen")}
              </Button>
            </DialogTrigger>
            <DialogContent className="max-w-xs">
              <DialogHeader>
                <DialogTitle>{t("catalog.dialogTitle")}</DialogTitle>
              </DialogHeader>
              <DialogDescription>{t("catalog.dialogBody")}</DialogDescription>
              <DialogFooter>
                <DialogClose asChild>
                  <Button variant="outline" data-testid="catalog-dialog-close">
                    {t("catalog.dialogClose")}
                  </Button>
                </DialogClose>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </Section>
      </div>
    </main>
  );
}
