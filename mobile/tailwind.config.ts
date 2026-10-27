import type { Config } from "tailwindcss";
import preset from "@localcamera/shared/tailwind-preset";
import animate from "tailwindcss-animate";

export default {
  presets: [preset],
  darkMode: "class",
  content: ["./index.html", "./src/**/*.{ts,tsx}", "../packages/shared/src/**/*.{ts,tsx}"],
  plugins: [animate],
} satisfies Config;
