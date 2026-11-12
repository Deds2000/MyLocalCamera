import { Shield } from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogTitle,
} from "./ui/alert-dialog";
import { Button } from "./ui/button";

export interface ApprovalRequest {
  name: string;
  model?: string;
  ip: string;
}

export interface ApprovalDialogProps {
  request: ApprovalRequest;
  /** Contagem regressiva a exibir (segundos; autoridade do timeout é o servidor). */
  seconds: number;
  onReject: () => void;
  onApproveOnce: () => void;
  onApproveAlways: () => void;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

/**
 * Esqueleto apresentacional da aprovação de dispositivo (ui-spec §2.8.1):
 * escudo, título com nome, modelo/IP, 3 botões e contagem de 30 s.
 * O comportamento (foco em Recusar, Esc/clique fora = recusar, notificação
 * Windows, autoridade do timeout) é implementado na F3-T05.
 */
export function ApprovalDialog({
  request,
  seconds,
  onReject,
  onApproveOnce,
  onApproveAlways,
  open = true,
  onOpenChange,
}: ApprovalDialogProps) {
  const { t } = useTranslation();

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent data-testid="approval-dialog" className="w-full max-w-md gap-4">
        <div className="flex items-center gap-3">
          <Shield
            data-testid="approval-shield"
            aria-hidden="true"
            className="h-8 w-8 shrink-0 text-brand"
          />
          <AlertDialogTitle data-testid="approval-title">
            {t("desktop.approval.title", { name: request.name })}
          </AlertDialogTitle>
        </div>
        <AlertDialogDescription data-testid="approval-body">
          {t("desktop.approval.body", { model: request.model ?? "", ip: request.ip })}
        </AlertDialogDescription>
        <p
          data-testid="approval-timer"
          className="text-xs text-text-muted"
        >
          {t("desktop.approval.timer", { seconds })}
        </p>
        <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
          <Button variant="ghost" onClick={onReject} data-testid="approval-reject">
            {t("desktop.approval.reject")}
          </Button>
          <Button variant="secondary" onClick={onApproveOnce} data-testid="approval-once">
            {t("desktop.approval.once")}
          </Button>
          <Button
            variant="default"
            onClick={onApproveAlways}
            data-testid="approval-always"
            className="bg-gradient-to-r from-brand-strong to-brand text-bg hover:opacity-90"
          >
            {t("desktop.approval.always")}
          </Button>
        </div>
      </AlertDialogContent>
    </AlertDialog>
  );
}
