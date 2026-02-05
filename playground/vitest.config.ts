import { defineConfig } from "vitest/config";
import { vize } from "@vizejs/vite-plugin";
import { playwright } from "@vitest/browser-playwright";

export default defineConfig({
  plugins: [vize()],
  resolve: {
    dedupe: ["vue"],
  },
  optimizeDeps: {
    include: ["vue", "@vue/test-utils"],
  },
  test: {
    browser: {
      enabled: true,
      provider: playwright(),
      instances: [{ browser: "chromium" }],
    },
    include: ["src/**/*.test.ts", "e2e/**/*.test.ts"],
  },
  server: {
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
  },
});
