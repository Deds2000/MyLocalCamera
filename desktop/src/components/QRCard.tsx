import { QRCodeSVG } from "qrcode.react";

import { cn } from "@/lib/utils";

export interface QRCardProps {
  uri: string;
  /** Hosts/IPs exibidos sob o QR (valores prontos; i18n é do consumidor). */
  hosts: string[];
  port: number;
  className?: string;
  size?: number;
  showNetworkDetails?: boolean;
}

/**
 * QR de 220 px com margem clara de 16 px (zona silenciosa, contraste para
 * leitura) + hosts/porta exibidos. O consumo do QR é da F3-T04.
 */
export function QRCard({ uri, hosts, port, className, size = 220, showNetworkDetails = true }: QRCardProps) {
  return (
    <div
      data-testid="qr-card"
      className={cn(
        "flex flex-col items-center gap-3 rounded-xl border border-glass-border bg-glass p-4 shadow-glass",
        className,
      )}
    >
      <div data-testid="qr-margin" className="rounded-lg bg-white p-4">
        <QRCodeSVG value={uri} size={size} />
      </div>
      {showNetworkDetails && (
        <div className="flex flex-col items-center gap-0.5 text-xs text-text-muted">
          <span data-testid="qr-hosts" className="font-mono">
            {hosts.join(" · ")}
          </span>
          <span data-testid="qr-port" className="font-mono">
            {port}
          </span>
        </div>
      )}
    </div>
  );
}
