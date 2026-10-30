// F2-T03 — Tela: Manual — ui/mobile.md §3.6.
//
// Campos: endereço (IPv4/hostname), porta (1–65535, padrão 48650) e código
// de pareamento (32 hex). Validação inline ao sair do campo. O atalho
// "Colar link" lê a área de transferência e preenche a partir de uma URI
// `localcamera://` (checagem simples scheme/host; validação completa é o
// parser Kotlin F2-T05a).

import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, ClipboardPaste } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useApp } from "@/state/AppProvider";
import { manualTarget } from "@/state/connection";
import { parsePairingUri } from "@localcamera/shared/protocol";
import { isPairingUri } from "@/qr/qr-scanner";
import { cn } from "@/lib/utils";

const DEFAULT_PORT = "48650";
const IP_OCTET = "(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])";
const ADDRESS = new RegExp(
  `^${IP_OCTET}(\\.${IP_OCTET}){3}$|^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?(\\.[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?)*$`,
);
const CODE = /^[0-9a-fA-F]{32}$/;

function isValidPort(value: string): boolean {
  return /^\d+$/.test(value) && Number(value) >= 1 && Number(value) <= 65535;
}

async function readClipboard(): Promise<string | null> {
  try {
    if (typeof navigator !== "undefined" && navigator.clipboard?.readText) {
      return await navigator.clipboard.readText();
    }
  } catch {
    // sem permissão de clipboard — ignora
  }
  return null;
}

interface FieldProps {
  id: string;
  label: string;
  value: string;
  placeholder?: string;
  inputMode?: "text" | "decimal";
  invalid: boolean;
  errorId?: string;
  onChange: (value: string) => void;
  onBlur: () => void;
}

function Field({
  id,
  label,
  value,
  placeholder,
  inputMode,
  invalid,
  errorId,
  onChange,
  onBlur,
}: FieldProps) {
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={id} className="text-sm text-text-muted">
        {label}
      </label>
      <input
        id={id}
        type="text"
        inputMode={inputMode}
        value={value}
        placeholder={placeholder}
        aria-invalid={invalid || undefined}
        aria-describedby={invalid ? errorId : undefined}
        onChange={(e) => onChange(e.target.value)}
        onBlur={onBlur}
        className={cn(
          "h-12 w-full rounded-md border bg-background px-3 text-base text-foreground",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
          invalid ? "border-danger" : "border-input",
        )}
      />
    </div>
  );
}

export function ManualScreen() {
  const { t } = useTranslation();
  const { dispatch } = useApp();

  const [address, setAddress] = useState("");
  const [port, setPort] = useState(DEFAULT_PORT);
  const [code, setCode] = useState("");
  const [touched, setTouched] = useState({ address: false, port: false, code: false });
  const [pasteInvalid, setPasteInvalid] = useState(false);

  const addressOk = ADDRESS.test(address.trim());
  const portOk = isValidPort(port.trim());
  const codeOk = CODE.test(code.trim());
  const allOk = addressOk && portOk && codeOk;

  const markTouched = (field: keyof typeof touched) =>
    setTouched((prev) => ({ ...prev, [field]: true }));

  const handlePaste = async () => {
    const text = (await readClipboard())?.trim();
    if (text === undefined || text === null || text === "") return;
    if (!isPairingUri(text)) {
      setPasteInvalid(true);
      return;
    }
    const res = parsePairingUri(text);
    if (res.ok) {
      setAddress(res.value.hosts[0] ?? "");
      setPort(String(res.value.port));
      setCode(res.value.token);
      setPasteInvalid(false);
    }
  };

  const handleConnect = (e: FormEvent) => {
    e.preventDefault();
    if (!allOk) return;
    dispatch({
      type: "CONNECT",
      target: manualTarget(address.trim(), Number(port.trim()), code.trim()),
      at: Date.now(),
    });
  };

  return (
    <main className="flex h-full flex-col gap-4 px-5 pb-6 pt-8">
      <header className="flex items-center justify-between">
        <Button
          variant="ghost"
          size="icon"
          aria-label={t("common.actions.back")}
          onClick={() => dispatch({ type: "ROUTE", route: "home" })}
        >
          <ArrowLeft aria-hidden="true" />
        </Button>
        <h1 className="text-lg font-semibold">{t("mobile.manual.title")}</h1>
        <Button
          variant="ghost"
          size="icon"
          aria-label={t("mobile.manual.paste")}
          onClick={() => void handlePaste()}
        >
          <ClipboardPaste aria-hidden="true" />
        </Button>
      </header>

      {pasteInvalid && (
        <div
          role="alert"
          className="rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-sm text-danger"
        >
          {t("mobile.scan.invalid")}
        </div>
      )}

      <form
        className="flex flex-1 flex-col gap-4 overflow-y-auto"
        onSubmit={handleConnect}
        noValidate
      >
        <div>
          <Field
            id="manual-address"
            label={t("mobile.manual.address")}
            value={address}
            inputMode="text"
            invalid={touched.address && !addressOk}
            errorId="manual-address-error"
            onChange={setAddress}
            onBlur={() => markTouched("address")}
          />
          {touched.address && !addressOk && (
            <p id="manual-address-error" className="mt-1 text-xs text-danger">
              {t("mobile.manual.invalidAddress")}
            </p>
          )}
        </div>

        <div>
          <Field
            id="manual-port"
            label={t("mobile.manual.port")}
            value={port}
            inputMode="decimal"
            invalid={touched.port && !portOk}
            errorId="manual-port-error"
            onChange={setPort}
            onBlur={() => markTouched("port")}
          />
          {touched.port && !portOk && (
            <p id="manual-port-error" className="mt-1 text-xs text-danger">
              {t("mobile.manual.invalidPort")}
            </p>
          )}
        </div>

        <div>
          <Field
            id="manual-code"
            label={t("mobile.manual.code")}
            value={code}
            inputMode="text"
            invalid={touched.code && !codeOk}
            errorId="manual-code-error"
            onChange={setCode}
            onBlur={() => markTouched("code")}
          />
          {touched.code && !codeOk && (
            <p id="manual-code-error" className="mt-1 text-xs text-danger">
              {t("mobile.manual.invalidCode")}
            </p>
          )}
        </div>
      </form>

      <div className="pb-2">
        <Button className="h-14 w-full" disabled={!allOk} onClick={handleConnect}>
          {t("mobile.manual.connect")}
        </Button>
      </div>
    </main>
  );
}
