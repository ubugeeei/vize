import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

// Toggle between @vitejs/plugin-vue and vite-plugin-vize
// In CI (production build), use official Vue compiler for stability
// In development, try to use Vize for testing
// FIXME: Temporarily disabled Vize due to template literal compilation bug
// Set USE_VIZE=true env var to enable Vize compiler
const USE_VIZE = process.env.USE_VIZE === "true";

async function getVuePlugin() {
  if (USE_VIZE) {
    try {
      const { vize } = await import("../npm/vite-plugin-vize/dist/index.js");
      console.log("[vite.config] Using Vize for Vue SFC compilation");
      return vize();
    } catch (e) {
      console.warn(
        "[vite.config] Failed to load Vize, falling back to @vitejs/plugin-vue:",
        e,
      );
      return vue();
    }
  }
  console.log("[vite.config] Using @vitejs/plugin-vue for Vue SFC compilation");
  return vue();
}

export default defineConfig(async () => {
  const vuePlugin = await getVuePlugin();

  return {
    base: process.env.CI ? "/play/" : "/",
    plugins: [vuePlugin, wasm(), topLevelAwait()],
    server: {
      headers: {
        "Cross-Origin-Opener-Policy": "same-origin",
        "Cross-Origin-Embedder-Policy": "require-corp",
      },
    },
    optimizeDeps: {
      exclude: ["vize-wasm"],
    },
  };
});
