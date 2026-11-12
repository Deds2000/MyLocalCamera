import { Camera, Check, ChevronDown, Copy, Globe, Minus, Square, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { resolveLanguage, savedLanguage, storeLanguage, type LanguageChoice } from "./lang";

/**
 * Titlebar de 40 px (F3-T01).
 *
 * Arrasto/duplo clique: `data-tauri-drag-region="deep"` no cabeçalho usa o
 * script nativo do Tauri (`drag.js`), que ignora `BUTTON` e alterna maximizar
 * no duplo clique. A pílula e o popover desligam a região com `"false"`.
 * Fora do Tauri (testes jsdom) as chamadas de janela são no-op.
 */

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const windowButton =
  "flex size-10 items-center justify-center text-text-muted transition-colors hover:bg-glass-strong hover:text-text";

export default function Titlebar() {
  const { t, i18n } = useTranslation();
  const [maximized, setMaximized] = useState(false);
  const [choice, setChoice] = useState<LanguageChoice>(() => savedLanguage());
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const win = isTauri ? getCurrentWindow() : null;

  useEffect(() => {
    if (!win) return;
    let active = true;
    const sync = () =>
      void win.isMaximized().then((m) => {
        if (active) setMaximized(m);
      });
    sync();
    window.addEventListener("resize", sync);
    return () => {
      active = false;
      window.removeEventListener("resize", sync);
    };
  }, [win]);

  useEffect(() => {
    if (!menuOpen) return;
    const onPointerDown = (e: Event) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) setMenuOpen(false);
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setMenuOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [menuOpen]);

  const selectLanguage = (next: LanguageChoice) => {
    setChoice(next);
    storeLanguage(next);
    void i18n.changeLanguage(resolveLanguage(next));
    setMenuOpen(false);
  };

  const choiceLabel =
    choice === "system"
      ? t("desktop.settings.languageSystem")
      : choice === "pt-BR"
        ? t("desktop.settings.languagePortuguese")
        : t("desktop.settings.languageEnglish");

  const items: Array<{ value: LanguageChoice; label: string }> = [
    { value: "system", label: t("desktop.settings.languageSystem") },
    { value: "pt-BR", label: t("desktop.settings.languagePortuguese") },
    { value: "en", label: t("desktop.settings.languageEnglish") },
  ];

  return (
    <header
      data-tauri-drag-region="deep"
      data-testid="titlebar"
      className="relative flex h-10 shrink-0 items-center border-b border-glass-border bg-raised"
    >
      <div className="flex items-center gap-2 pl-4">
        <Camera className="size-5 text-brand" aria-hidden />
        <span className="text-sm font-semibold tracking-tight">
          <span className="text-text">{t("desktop.titlebar.logoLocal")}</span>
          <span className="text-brand">{t("desktop.titlebar.logoCamera")}</span>
        </span>
      </div>

      <div className="flex flex-1 justify-center px-3">
        <div
          data-tauri-drag-region="false"
          data-testid="status-pill"
          className="flex items-center gap-1.5 rounded-full border border-glass-border bg-glass px-3 py-0.5 text-xs text-text-muted"
        >
          <span className="size-1.5 rounded-full bg-text-muted" aria-hidden />
          {t("common.state.idle")}
        </div>
      </div>

      <div className="flex items-center gap-0.5 pr-1" ref={menuRef}>
        <div className="relative">
          <button
            type="button"
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            data-testid="language-trigger"
            onClick={() => setMenuOpen((open) => !open)}
            className={windowButton}
          >
            <Globe className="size-4" aria-hidden />
            <span className="max-w-24 truncate text-xs">{choiceLabel}</span>
            <ChevronDown className="size-3.5 opacity-70" aria-hidden />
          </button>
          {menuOpen && (
            <div
              data-tauri-drag-region="false"
              role="menu"
              data-testid="language-menu"
              className="absolute right-0 top-10 z-50 min-w-40 rounded-md border border-glass-border bg-glass-strong p-1 shadow-overlay"
            >
              {items.map((item) => (
                <button
                  key={item.value}
                  type="button"
                  role="menuitem"
                  onClick={() => selectLanguage(item.value)}
                  className="flex w-full items-center justify-between gap-3 rounded-sm px-2.5 py-1.5 text-left text-sm text-text hover:bg-glass"
                >
                  {item.label}
                  {choice === item.value && <Check className="size-4 text-brand" aria-hidden />}
                </button>
              ))}
            </div>
          )}
        </div>

        <button type="button" aria-label={t("desktop.window.minimize")} onClick={() => void win?.minimize()} className={windowButton}>
          <Minus className="size-4" aria-hidden />
        </button>
        <button
          type="button"
          aria-label={maximized ? t("desktop.window.restore") : t("desktop.window.maximize")}
          onClick={() => void (maximized ? win?.unmaximize() : win?.maximize())}
          className={windowButton}
        >
          {maximized ? <Copy className="size-4" aria-hidden /> : <Square className="size-4" aria-hidden />}
        </button>
        <button
          type="button"
          aria-label={t("desktop.window.close")}
          onClick={() => void win?.close()}
          className="flex size-10 items-center justify-center text-text-muted transition-colors hover:bg-danger hover:text-bg"
        >
          <X className="size-4" aria-hidden />
        </button>
      </div>
    </header>
  );
}
