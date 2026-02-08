/**
 * Vize - High-performance Vue.js toolchain in Rust
 *
 * This package provides:
 * - CLI binary for compilation, linting, and formatting
 * - Configuration utilities for programmatic use
 */

// Types
export type {
  VizeConfig,
  CompilerConfig,
  VitePluginConfig,
  LinterConfig,
  TypeCheckerConfig,
  FormatterConfig,
  LspConfig,
  MuseaConfig,
  MuseaVrtConfig,
  MuseaA11yConfig,
  MuseaAutogenConfig,
  GlobalTypesConfig,
  GlobalTypeDeclaration,
  LoadConfigOptions,
  ConfigEnv,
  UserConfigExport,
  MaybePromise,
  RuleSeverity,
  RuleCategory,
} from "./types.js";

// Config utilities
export { defineConfig, loadConfig, normalizeGlobalTypes } from "./config.js";
