/**
 * @vizejs/nuxt - Nuxt module for Vize
 *
 * Provides:
 * - Compiler: Vue SFC compilation via Vite plugin
 * - Musea: Component gallery with Nuxt mock support
 * - Linter: `vize lint` CLI command (via `vize` bin)
 * - Type Checker: `vize check` CLI command (via `vize` bin)
 */

import { defineNuxtModule } from "@nuxt/kit";
import vize from "@vizejs/vite-plugin";
import { musea } from "@vizejs/vite-plugin-musea";
import type { MuseaOptions } from "@vizejs/vite-plugin-musea";
import type { NuxtMuseaOptions } from "@vizejs/musea-nuxt";

export interface VizeNuxtOptions {
  /**
   * Musea gallery options.
   * Set to `false` to disable musea.
   */
  musea?: MuseaOptions | false;

  /**
   * Nuxt mock options for musea gallery.
   * NOTE: In Nuxt context, nuxtMusea mocks are NOT added as a global Vite plugin
   * because they would intercept `#imports` resolution and break Nuxt's internals.
   * Real Nuxt composables are available via Nuxt's own plugin pipeline.
   */
  nuxtMusea?: NuxtMuseaOptions;
}

export default defineNuxtModule<VizeNuxtOptions>({
  meta: {
    name: "@vizejs/nuxt",
    configKey: "vize",
  },
  defaults: {
    musea: {
      include: ["**/*.art.vue"],
      inlineArt: false,
    },
    nuxtMusea: {
      route: { path: "/" },
    },
  },
  setup(options, nuxt) {
    nuxt.options.vite.plugins = nuxt.options.vite.plugins || [];

    // Compiler
    nuxt.options.vite.plugins.push(vize());

    // Musea gallery (without nuxtMusea mock layer)
    // In Nuxt context, real composables/components are already available
    // via Nuxt's own Vite plugins. Adding nuxtMusea globally would shadow
    // Nuxt's #imports resolution and break the app.
    if (options.musea !== false) {
      const museaBasePath = options.musea && typeof options.musea === "object" && "basePath" in options.musea
        ? (options.musea as Record<string, unknown>).basePath as string
        : "/__musea__";
      nuxt.options.vite.plugins.push(...musea(options.musea || {}));

      // Print Musea Gallery URL after dev server starts
      nuxt.hook("listen", (_server: unknown, listener: { url: string }) => {
        const url = listener.url?.replace(/\/$/, "") || "http://localhost:3000";
        console.log(`  \x1b[36m➜\x1b[0m  \x1b[1mMusea Gallery:\x1b[0m \x1b[36m${url}${museaBasePath}\x1b[0m`);
      });
    }
  },
});

// Re-export types for convenience
export type { MuseaOptions } from "@vizejs/vite-plugin-musea";
export type { NuxtMuseaOptions } from "@vizejs/musea-nuxt";
