import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import path from "path";
import { defineConfig } from "vite";

// Mirrors the codeless-ui setup: React 19 + Tailwind v4 (Vite plugin)
// + the `@/` alias. The dev server proxies `/v1/*` and `/metrics` to
// the local hackline-gateway so the UI can use same-origin URLs and
// EventSource works without CORS gymnastics.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  server: {
    port: 1430,
    strictPort: true,
    proxy: {
      "/v1": {
        target: process.env.HACKLINE_GATEWAY_URL ?? "http://127.0.0.1:8080",
        changeOrigin: true,
      },
      "/metrics": {
        target: process.env.HACKLINE_GATEWAY_URL ?? "http://127.0.0.1:8080",
        changeOrigin: true,
      },
    },
  },
  build: {
    target: "es2020",
    chunkSizeWarningLimit: 1500,
  },
});
