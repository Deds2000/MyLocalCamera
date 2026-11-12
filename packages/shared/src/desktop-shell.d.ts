/**
 * F3-T14b: `pseudo.test.tsx` renderiza o shell do desktop pelo alias de teste
 * `@localcamera/desktop-shell` (vitest.config.ts → desktop/src/app/Shell.tsx).
 * O package `desktop` não é dependência do `shared`; esta declaração existe só
 * para o typecheck do teste (nenhum código de produção a importa).
 */
declare module "@localcamera/desktop-shell" {
  import type { ComponentType } from "react";
  const Shell: ComponentType;
  export default Shell;
}
