import type { Config } from "tailwindcss";

export default {
  content: ["./templates/**/*.html"],
  theme: {
    extend: {
      colors: {
        "surface": "#f9fafb",
        "surface-alt": "#e5e7eb",
        "surfaceDark": "#101827",
        "surfaceDark-alt": "#1f2937",
      }
    },
  },
  plugins: [],
} satisfies Config;
