export type {
  VizeConfig,
  LoadConfigOptions,
  ConfigEnv,
  UserConfigExport,
} from "../../vize/src/types.js";

export interface SfcCompileOptionsNapi {
  filename?: string;
  sourceMap?: boolean;
  ssr?: boolean;
  scopeId?: string;
}

export interface SfcCompileResultNapi {
  code: string;
  css?: string;
  errors: string[];
  warnings: string[];
}

export type CompileSfcFn = (
  source: string,
  options?: SfcCompileOptionsNapi,
) => SfcCompileResultNapi;

export interface VizeOptions {
  /**
   * Files to include in compilation
   * @default /\.vue$/
   */
  include?: string | RegExp | (string | RegExp)[];

  /**
   * Files to exclude from compilation
   * @default /node_modules/
   */
  exclude?: string | RegExp | (string | RegExp)[];

  /**
   * Force production mode
   * @default auto-detected from Vite config
   */
  isProduction?: boolean;

  /**
   * Enable SSR mode
   * @default false
   */
  ssr?: boolean;

  /**
   * Enable source map generation
   * @default true in development, false in production
   */
  sourceMap?: boolean;

  /**
   * Enable Vapor mode compilation
   * @default false
   */
  vapor?: boolean;

  /**
   * Root directory to scan for .vue files
   * @default Vite's root
   */
  root?: string;

  /**
   * Glob patterns to scan for .vue files during pre-compilation
   * @default ['**\/*.vue']
   */
  scanPatterns?: string[];

  /**
   * Glob patterns to ignore during pre-compilation
   * @default ['node_modules/**', 'dist/**', '.git/**']
   */
  ignorePatterns?: string[];

  /**
   * Config file search mode
   * - 'root': Search only in the project root directory
   * - 'auto': Search from cwd upward until finding a config file
   * - false: Disable config file loading
   * @default 'root'
   */
  configMode?: "root" | "auto" | false;

  /**
   * Custom config file path (overrides automatic search)
   */
  configFile?: string;

  /**
   * Handle .vue files in node_modules (on-demand compilation).
   * When true, vize will compile .vue files from node_modules that other plugins
   * (like vite-plugin-vue-inspector) may import directly.
   * Set to false if another Vue plugin (e.g. Nuxt) handles node_modules .vue files.
   * @default true
   */
  handleNodeModulesVue?: boolean;

  /**
   * Enable debug logging
   * @default false
   */
  debug?: boolean;
}

export interface CompiledModule {
  code: string;
  css?: string;
  scopeId: string;
  hasScoped: boolean;
  templateHash?: string;
  styleHash?: string;
  scriptHash?: string;
}

export interface BatchFileInput {
  path: string;
  source: string;
}

export interface BatchFileResult {
  path: string;
  code: string;
  css?: string;
  scopeId: string;
  hasScoped: boolean;
  errors: string[];
  warnings: string[];
  templateHash?: string;
  styleHash?: string;
  scriptHash?: string;
}

export interface BatchCompileOptionsNapi {
  ssr?: boolean;
  threads?: number;
}

export interface BatchCompileResultWithFiles {
  results: BatchFileResult[];
  successCount: number;
  failedCount: number;
  timeMs: number;
}

export type CompileSfcBatchWithResultsFn = (
  files: BatchFileInput[],
  options?: BatchCompileOptionsNapi,
) => BatchCompileResultWithFiles;
