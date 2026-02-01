import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";
import { playwright } from "@vitest/browser-playwright";

async function getVuePlugin() {
  try {
    const { vize } = await import("@vizejs/vite-plugin");
    console.log("[vitest.config] Using Vize for Vue SFC compilation");
    return vize();
  } catch (e) {
    console.warn(
      "[vitest.config] Failed to load Vize, falling back to @vitejs/plugin-vue:",
      e,
    );
    return vue();
  }
}

export default defineConfig(async () => {
  const vuePlugin = await getVuePlugin();

  return {
    plugins: [vuePlugin],
    resolve: {
      dedupe: ["vue", "@vue/runtime-core", "@vue/runtime-dom", "@vue/reactivity"],
    },
    optimizeDeps: {
      exclude: ["vue", "@vue/runtime-core", "@vue/runtime-dom", "@vue/reactivity"],
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
  };
});
