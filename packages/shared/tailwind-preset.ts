import type { Config } from "tailwindcss";

/** Tokens (hex) do design system — objeto simples para uso em testes. */
export const tokens = {
  bg: "#070B14",
  bgRaised: "#0C1322",
  text: "#E6EDF7",
  textMuted: "#8FA1BD",
  accent: "#22D3EE",
  accentStrong: "#06B6D4",
  accentBlue: "#3B82F6",
  ok: "#34D399",
  warn: "#FBBF24",
  // rose-400 (400-level, como ok/warn): #F43F5E falhava WCAG AA (4.39:1)
  // sobre `bg-glass-strong`; #FB7185 passa (~6:1) — F2-T14 UT-2.14.
  danger: "#FB7185",
} as const;

/**
 * Conflito de nomes: `accent` é o token SEMÂNTICO do shadcn (hsl(var(--accent))),
 * então a cor de DESTAQUE do app (hex) foi exposta como `brand`
 * (brand = accent do app, brand.strong = accent-strong, brand.blue = accent-blue).
 */
const preset: Config = {
  // O tipo `Config` do Tailwind exige `content`; os arquivos vigentes são
  // definidos na config de cada app consumidor (desktop/mobile).
  content: [],
  theme: {
    extend: {
      colors: {
        bg: { DEFAULT: tokens.bg, raised: tokens.bgRaised },
        glass: {
          DEFAULT: "rgb(255 255 255 / 0.05)",
          strong: "rgb(255 255 255 / 0.09)",
          border: "rgb(255 255 255 / 0.10)",
        },
        // shadcn (semântico, via CSS var)
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        // destaque do app (hex do design system)
        brand: {
          DEFAULT: tokens.accent,
          strong: tokens.accentStrong,
          blue: tokens.accentBlue,
        },
        ok: tokens.ok,
        warn: tokens.warn,
        danger: tokens.danger,
        text: { DEFAULT: tokens.text, muted: tokens.textMuted },
        // demais cores semânticas do shadcn
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
      },
      borderRadius: {
        sm: "8px",
        md: "12px",
        lg: "16px",
        xl: "24px",
      },
      boxShadow: {
        glass: "0 8px 32px rgb(0 0 0 / .35), inset 0 1px 0 rgb(255 255 255 / .06)",
        overlay: "0 24px 64px rgb(0 0 0 / .55), inset 0 1px 0 rgb(255 255 255 / .08)",
        "glow-accent": "0 0 24px rgb(34 211 238 / .25)",
      },
      fontFamily: {
        sans: ['"Inter Variable"', "Inter", "system-ui", "sans-serif"],
        mono: ['"JetBrains Mono Variable"', "ui-monospace", "monospace"],
      },
      keyframes: {
        "pulse-ring": {
          "0%": { transform: "scale(1)", opacity: "0.6" },
          "100%": { transform: "scale(1.8)", opacity: "0" },
        },
      },
      animation: {
        "pulse-ring": "pulse-ring 1.6s ease-out infinite",
      },
    },
  },
};

export default preset;
