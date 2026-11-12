import type { Transition, Variants } from "framer-motion";

/** Durações do ui-spec §1.7 (em segundos): fast 120 ms / base 200 ms / slow 320 ms. */
export const durations = { fast: 0.12, base: 0.2, slow: 0.32 } as const;

/** Easing do ui-spec §1.7: cubic-bezier(0.22, 1, 0.36, 1). */
export const easing = [0.22, 1, 0.36, 1] as const;

/** Mola padrão do ui-spec §1.7. */
export const springDefault = { type: "spring", stiffness: 300, damping: 30 } as const;

/** `prefers-reduced-motion`: só fade, 0–80 ms, sem deslocamento. */
export const reducedFadeIn: Transition = { duration: 0.08, ease: "linear" };
export const reducedFadeOut: Transition = { duration: 0.04, ease: "linear" };

function reducedFadeVariants(): Variants {
  return {
    initial: { opacity: 0 },
    animate: { opacity: 1, transition: reducedFadeIn },
    exit: { opacity: 0, transition: reducedFadeOut },
  };
}

/** Troca de seção do painel: fade + 8 px (ui-spec §1.7). */
export function sectionVariants(reducedMotion = false): Variants {
  if (reducedMotion) return reducedFadeVariants();
  return {
    initial: { opacity: 0, y: 8 },
    animate: { opacity: 1, y: 0, transition: { duration: durations.base, ease: easing } },
    exit: { opacity: 0, y: -8, transition: { duration: durations.fast, ease: easing } },
  };
}

/** Cartões: fade + scale 0.98 → 1. */
export function cardVariants(reducedMotion = false): Variants {
  if (reducedMotion) return reducedFadeVariants();
  return {
    initial: { opacity: 0, scale: 0.98 },
    animate: { opacity: 1, scale: 1, transition: springDefault },
    exit: { opacity: 0, scale: 0.98, transition: reducedFadeOut },
  };
}

/** Diálogos: fade + scale 0.96 → 1. */
export function dialogVariants(reducedMotion = false): Variants {
  if (reducedMotion) return reducedFadeVariants();
  return {
    initial: { opacity: 0, scale: 0.96 },
    animate: { opacity: 1, scale: 1, transition: springDefault },
    exit: { opacity: 0, scale: 0.96, transition: reducedFadeOut },
  };
}
