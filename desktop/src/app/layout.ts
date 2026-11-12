/**
 * Breakpoints do layout por largura da JANELA (F3-T01):
 * ≥ 1280 → painel 380 px · 1100–1279 → 340 px · 960–1099 → 300 px.
 */

export function panelWidth(windowWidth: number): number {
  return windowWidth >= 1280 ? 380 : windowWidth >= 1100 ? 340 : 300;
}

export function previewPadding(windowWidth: number): number {
  return windowWidth >= 1100 ? 24 : 16;
}
