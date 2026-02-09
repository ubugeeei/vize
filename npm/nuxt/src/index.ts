/**
 * @vizejs/nuxt - All-in-one Vize integration for Nuxt
 *
 * Provides:
 * - Compiler: Vue SFC compilation via Vite plugin
 * - Musea: Component gallery with Nuxt mock support
 * - Linter: `vize lint` CLI command
 * - Type Checker: `vize check` CLI command
 */

// Compiler (Vite plugin)
export { default as vize } from "@vizejs/vite-plugin";
export type { VizeOptions } from "@vizejs/vite-plugin";

// Musea (Gallery)
export { musea } from "@vizejs/vite-plugin-musea";
export type { MuseaOptions } from "@vizejs/vite-plugin-musea";

// Musea Nuxt (Nuxt mocks for gallery)
export { nuxtMusea } from "@vizejs/musea-nuxt";
export type { NuxtMuseaOptions } from "@vizejs/musea-nuxt";

// Config utilities
export { defineConfig, loadConfig } from "vize";
export type { VizeConfig, UserConfigExport } from "vize";
