import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useStore } from "zustand";
import type { StoreApi } from "zustand/vanilla";
import { Clock3, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  AlertDialog, AlertDialogCancel, AlertDialogContent,
  AlertDialogDescription, AlertDialogFooter, AlertDialogTitle, AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import type { TrustedDevice } from "@/lib/tauri";
import { selectTrustedDevices, type AppStore } from "@/store";

interface TrustedListProps { store: StoreApi<AppStore> }

function relativeSeen(seconds: number, locale: string): string {
  const delta = Math.floor(seconds) * 1000 - Date.now();
  const units: Array<[Intl.RelativeTimeFormatUnit, number]> = [
    ["year", 31_536_000_000], ["month", 2_592_000_000], ["day", 86_400_000],
    ["hour", 3_600_000], ["minute", 60_000], ["second", 1000],
  ];
  const fallback: [Intl.RelativeTimeFormatUnit, number] = ["second", 1000];
  const [unit, size] = units.find(([, size]) => Math.abs(delta) >= size) ?? fallback;
  return new Intl.RelativeTimeFormat(locale, { numeric: "auto" }).format(Math.round(delta / size), unit);
}

function DeviceRow({ device, store, onRemoved }: { device: TrustedDevice; store: StoreApi<AppStore>; onRemoved: () => void }) {
  const { t, i18n } = useTranslation();
  const [open, setOpen] = useState(false);
  const [pending, setPending] = useState(false);
  const [refreshPending, setRefreshPending] = useState(false);
  const [refreshResult, setRefreshResult] = useState<"present" | null>(null);
  const errorCode = useStore(store, (state) => state.trust.errorCode);
  const confirmed = async () => {
    setPending(true);
    await store.getState().revokeTrustedDevice(device.deviceId);
    const failed = store.getState().trust.errorCode !== null;
    setPending(false);
    setOpen(failed);
    if (!failed) onRemoved();
  };
  const refresh = async () => {
    setRefreshPending(true);
    setRefreshResult(null);
    await store.getState().loadTrustedDevices();
    const state = store.getState().trust;
    const failed = state.errorCode === "TRUST_LIST_FAILED";
    setRefreshPending(false);
    if (failed) return;
    if (state.trusted.some((trustedDevice) => trustedDevice.deviceId === device.deviceId)) {
      setRefreshResult("present");
      return;
    }
    setOpen(false);
    onRemoved();
  };
  return (
    <li className="flex items-center gap-3 rounded-lg border border-glass-border bg-glass p-3">
      <Clock3 aria-hidden="true" className="size-4 shrink-0 text-text-muted" />
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">{device.name}</p>
        <p className="truncate text-xs text-text-muted">
          {[device.model, t("desktop.trusted.lastSeen", { when: relativeSeen(device.lastSeen, i18n.language) }), device.lastIp].filter(Boolean).join(" · ")}
        </p>
      </div>
      <AlertDialog open={open} onOpenChange={(value) => !pending && setOpen(value)}>
        <AlertDialogTrigger asChild>
          <Button type="button" size="icon" variant="ghost" aria-label={t("desktop.trusted.remove", { name: device.name })} disabled={pending}>
            <Trash2 aria-hidden="true" />
          </Button>
        </AlertDialogTrigger>
        <AlertDialogContent>
          <AlertDialogTitle className="break-words">{t("desktop.trusted.remove", { name: device.name })}</AlertDialogTitle>
          <AlertDialogDescription className="break-words">{t("desktop.trusted.removeConfirm", { name: device.name })}</AlertDialogDescription>
          {errorCode === "TRUST_REVOKE_FAILED" && <p role="alert" className="break-words text-sm text-danger">{t("desktop.trusted.revokeFailed")}</p>}
          {errorCode === "TRUST_LIST_FAILED" && <p role="alert" className="break-words text-sm text-danger">{t("desktop.trusted.loadFailed")}</p>}
          {refreshResult === "present" && <p role="status" className="break-words text-sm text-text-muted">{t("desktop.trusted.refreshPresent")}</p>}
          <AlertDialogFooter>
            {(errorCode === "TRUST_REVOKE_FAILED" || errorCode === "TRUST_LIST_FAILED") && <Button type="button" variant="outline" disabled={pending || refreshPending} onClick={() => void refresh()}>
              {refreshPending ? t("desktop.trusted.refreshing") : t("desktop.trusted.refresh")}
            </Button>}
            <AlertDialogCancel disabled={pending || refreshPending}>{t("common.actions.cancel")}</AlertDialogCancel>
            <Button type="button" variant="destructive" className="bg-danger text-bg hover:bg-danger/90" disabled={pending || refreshPending || errorCode !== null} onClick={() => void confirmed()}>
              {pending ? t("desktop.trusted.removing") : t("common.actions.confirm")}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </li>
  );
}

export function TrustedList({ store }: TrustedListProps) {
  const { t } = useTranslation();
  const trusted = useStore(store, selectTrustedDevices);
  const errorCode = useStore(store, (state) => state.trust.errorCode);
  const [loading, setLoading] = useState(true);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const retryLoad = useCallback(async () => {
    setLoading(true);
    await store.getState().loadTrustedDevices();
    setLoading(false);
  }, [store]);
  useEffect(() => {
    let active = true;
    void store.getState().loadTrustedDevices().then(() => {
      if (active) setLoading(false);
    });
    return () => { active = false; };
  }, [store]);
  return (
    <section aria-labelledby="trusted-title" className="space-y-3">
      <h3 id="trusted-title" ref={headingRef} tabIndex={-1} className="text-sm font-semibold focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">{t("desktop.trusted.title")}</h3>
      {loading ? <p role="status" className="text-sm text-text-muted">{t("desktop.trusted.loading")}</p> : <>
        {errorCode === "TRUST_LIST_FAILED" && <div role="alert" className="space-y-2"><p className="text-sm text-danger">{t("desktop.trusted.loadFailed")}</p><Button type="button" variant="outline" onClick={() => void retryLoad()}>{t("common.actions.retry")}</Button></div>}
        {trusted.length === 0 ? errorCode !== "TRUST_LIST_FAILED" && <p className="rounded-lg border border-glass-border bg-glass p-3 text-sm text-text-muted">{t("desktop.trusted.empty")}</p> : (
          <ul className="space-y-2" aria-label={t("desktop.trusted.title")}>
            {trusted.map((device) => <DeviceRow key={device.deviceId} device={device} store={store} onRemoved={() => headingRef.current?.focus()} />)}
          </ul>
        )}
      </>}
    </section>
  );
}
