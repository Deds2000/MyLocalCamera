import { Toaster as Sonner, type ToasterProps } from "sonner";

import { cn } from "@/lib/utils";

/**
 * Toaster Sonner configurado para o canto inferior esquerdo (ui-spec §2.8.4:
 * não cobre o preview). O toaster é montado no App na F3-T04/T14a.
 */
const Toaster = ({ className, position = "bottom-left", ...props }: ToasterProps) => {
  return (
    <Sonner
      theme="dark"
      position={position}
      className={cn("toaster group", className)}
      {...props}
    />
  );
};

export { Toaster };
