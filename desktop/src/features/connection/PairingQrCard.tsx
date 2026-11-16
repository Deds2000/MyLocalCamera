import { useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Copy, Eye, EyeOff, RefreshCw } from "lucide-react";
import { useStore } from "zustand";
import type { StoreApi } from "zustand/vanilla";
import { toast } from "sonner";
import { parsePairingUri } from "@localcamera/shared/protocol";
import { QRCard } from "@/components/QRCard";
import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogTrigger,
  AlertDialogFooter,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { appStore, type AppStore } from "@/store";
import { filterInterfaces, pairingLinkForInterface } from "./interface-filter";

interface PairingQrCardProps {
  showCode?: boolean;
  store?: StoreApi<AppStore>;
  compact?: boolean;
  showInterfaceSelector?: boolean;
}

const ALL_INTERFACES = "__all_interfaces__";

export function PairingQrCard({
  showCode = true,
  store = appStore,
  compact = false,
  showInterfaceSelector = true,
}: PairingQrCardProps) {
  const { t } = useTranslation();
  const interfaceLabelId = useId();
  const info = useStore(store, (state) => state.connectionInfo);
  const loading = useStore(store, (state) => state.connectionInfoLoading);
  const errorCode = useStore(store, (state) => state.connectionInfoErrorCode);
  const savedInterface = useStore(store, (state) => state.settings.networkInterface);
  const setNetworkInterface = useStore(store, (state) => state.setNetworkInterface);
  const [showFullCode, setShowFullCode] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [regenerating, setRegenerating] = useState(false);

  const filtered = useMemo(
    () => filterInterfaces(info?.interfaces ?? [], savedInterface),
    [info, savedInterface],
  );
  const selectedInterface = savedInterface && filtered.shown.some((item) => item.ip === savedInterface)
    ? savedInterface
    : null;
  const pairingLink = useMemo(
    () => (info ? pairingLinkForInterface(info, selectedInterface) : null),
    [info, selectedInterface],
  );
  const parsedPairing = pairingLink ? parsePairingUri(pairingLink.uri) : null;
  const fullCode = parsedPairing?.ok ? parsedPairing.value.token : null;

  const confirmRegenerate = async () => {
    setConfirmOpen(false);
    setRegenerating(true);
    await store.getState().regeneratePairingToken();
    setRegenerating(false);
    if (store.getState().connectionInfoErrorCode) toast.error(t("common.errors.generic"));
  };

  const copyCode = async () => {
    if (!fullCode) return;
    try {
      await navigator.clipboard.writeText(fullCode);
      toast.success(t("common.actions.copied"));
    } catch {
      toast.error(t("common.errors.generic"));
    }
  };

  if (!info || !pairingLink) {
    return (
      <div className="flex flex-col items-center gap-3" aria-busy={loading}>
        {loading ? (
          <div data-testid="qr-loading" className="size-56 animate-pulse rounded-xl bg-glass" />
        ) : (
          <>
            <p role="status" className="text-center text-sm text-text-muted">
              {errorCode ? t("common.errors.generic") : t("common.errors.noNetwork")}
            </p>
            {errorCode && (
              <Button variant="secondary" size="sm" onClick={() => void store.getState().loadConnectionInfo()}>
                {t("common.actions.retry")}
              </Button>
            )}
          </>
        )}
      </div>
    );
  }

  return (
    <div className={`flex w-full flex-col items-center ${compact ? "gap-2" : "gap-3"}`}>
      {showInterfaceSelector && filtered.shown.length > 1 && (
        <div className="w-full space-y-1.5">
          <span id={interfaceLabelId} className="text-xs font-medium text-text-muted">
            {t("desktop.connection.interface")}
          </span>
          <Select
            value={selectedInterface ?? ALL_INTERFACES}
            onValueChange={(value) => setNetworkInterface(value === ALL_INTERFACES ? null : value)}
          >
            <SelectTrigger aria-labelledby={interfaceLabelId} data-testid="interface-select">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL_INTERFACES}>{t("desktop.connection.interfaceAll")}</SelectItem>
              {filtered.shown.map((networkInterface) => (
                <SelectItem key={networkInterface.ip} value={networkInterface.ip}>
                  {networkInterface.name} · {networkInterface.ip}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      )}

      <div role="img" aria-label={t("desktop.connection.qrLabel")}>
        <QRCard
          uri={pairingLink.uri}
          hosts={pairingLink.hosts}
          port={info.effectivePort}
          size={compact ? 140 : 220}
          showNetworkDetails={false}
          className="p-1.5"
        />
      </div>
      <p data-testid="network-caption" className="text-center text-xs text-text-muted">
        {t("desktop.empty.network", { ip: pairingLink.ip, port: info.effectivePort })}
      </p>

      <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
        <AlertDialogTrigger asChild>
          <Button
            type="button"
            variant="secondary"
            size="sm"
            disabled={regenerating || loading}
            data-testid="regenerate-qr"
          >
            <RefreshCw aria-hidden="true" />
            {t("desktop.connection.regenerate")}
          </Button>
        </AlertDialogTrigger>
        <AlertDialogContent data-testid="regenerate-dialog">
          <AlertDialogTitle>{t("desktop.connection.regenerate")}</AlertDialogTitle>
          <AlertDialogDescription>{t("desktop.connection.regenerateConfirm")}</AlertDialogDescription>
          <AlertDialogFooter>
            <AlertDialogCancel asChild>
              <Button variant="outline">{t("common.actions.cancel")}</Button>
            </AlertDialogCancel>
            <AlertDialogAction asChild>
              <Button
                onClick={(event) => {
                  event.preventDefault();
                  void confirmRegenerate();
                }}
                disabled={regenerating}
                data-testid="confirm-regenerate"
              >
                {t("common.actions.confirm")}
              </Button>
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {showCode && (
        <div className="w-full space-y-1.5">
          <span className="text-xs font-medium text-text-muted">{t("desktop.connection.code")}</span>
          <div className="flex items-center gap-2 rounded-md border border-glass-border bg-glass px-2 py-1.5">
            <code data-testid="pairing-code" className="min-w-0 flex-1 truncate font-mono text-xs">
              {showFullCode && fullCode ? fullCode : info.pairCodeMasked}
            </code>
            <Button
              type="button"
              size="icon"
              variant="ghost"
              aria-label={t(showFullCode ? "desktop.connection.hideCode" : "desktop.connection.showCode")}
              aria-pressed={showFullCode}
              disabled={!fullCode}
              onClick={() => setShowFullCode((visible) => !visible)}
            >
              {showFullCode ? <EyeOff aria-hidden="true" /> : <Eye aria-hidden="true" />}
            </Button>
            <Button
              type="button"
              size="icon"
              variant="ghost"
              aria-label={t("common.actions.copy")}
              disabled={!fullCode}
              onClick={() => void copyCode()}
            >
              <Copy aria-hidden="true" />
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
