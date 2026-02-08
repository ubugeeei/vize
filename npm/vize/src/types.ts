// ============================================================================
// Dynamic config support
// ============================================================================

export type MaybePromise<T> = T | Promise<T>;

export interface ConfigEnv {
  mode: string;
  command: "serve" | "build" | "check" | "lint" | "fmt";
  isSsrBuild?: boolean;
}

export type UserConfigExport = VizeConfig | ((env: ConfigEnv) => MaybePromise<VizeConfig>);

// ============================================================================
// Rule severity
// ============================================================================

export type RuleSeverity = "off" | "warn" | "error";

export type RuleCategory = "correctness" | "suspicious" | "style" | "perf" | "a11y" | "security";

// ============================================================================
// VizeConfig
// ============================================================================

/**
 * Vize configuration options
 */
export interface VizeConfig {
  /**
   * Vue compiler options
   */
  compiler?: CompilerConfig;

  /**
   * Vite plugin options
   */
  vite?: VitePluginConfig;

  /**
   * Linter options
   */
  linter?: LinterConfig;

  /**
   * Type checker options
   */
  typeChecker?: TypeCheckerConfig;

  /**
   * Formatter options
   */
  formatter?: FormatterConfig;

  /**
   * LSP options
   */
  lsp?: LspConfig;

  /**
   * Musea component gallery options
   */
  musea?: MuseaConfig;

  /**
   * Global type declarations
   */
  globalTypes?: GlobalTypesConfig;
}

// ============================================================================
// CompilerConfig
// ============================================================================

/**
 * Compiler configuration
 */
export interface CompilerConfig {
  /**
   * Compilation mode
   * @default 'module'
   */
  mode?: "module" | "function";

  /**
   * Enable Vapor mode compilation
   * @default false
   */
  vapor?: boolean;

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
   * Prefix template identifiers with _ctx
   * @default false
   */
  prefixIdentifiers?: boolean;

  /**
   * Hoist static nodes
   * @default true
   */
  hoistStatic?: boolean;

  /**
   * Cache v-on handlers
   * @default true
   */
  cacheHandlers?: boolean;

  /**
   * Enable TypeScript parsing in <script> blocks
   * @default true
   */
  isTs?: boolean;

  /**
   * Script file extension for generated output
   * @default 'ts'
   */
  scriptExt?: "ts" | "js";

  /**
   * Module name for runtime imports
   * @default 'vue'
   */
  runtimeModuleName?: string;

  /**
   * Global variable name for runtime (IIFE builds)
   * @default 'Vue'
   */
  runtimeGlobalName?: string;
}

// ============================================================================
// VitePluginConfig
// ============================================================================

/**
 * Vite plugin configuration
 */
export interface VitePluginConfig {
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
   * Glob patterns to scan for .vue files during pre-compilation
   * @default ['**\/*.vue']
   */
  scanPatterns?: string[];

  /**
   * Glob patterns to ignore during pre-compilation
   * @default ['node_modules/**', 'dist/**', '.git/**']
   */
  ignorePatterns?: string[];
}

// ============================================================================
// LinterConfig
// ============================================================================

/**
 * Linter configuration
 */
export interface LinterConfig {
  /**
   * Enable linting
   */
  enabled?: boolean;

  /**
   * Rules to enable/disable
   */
  rules?: Record<string, RuleSeverity>;

  /**
   * Category-level severity overrides
   */
  categories?: Partial<Record<RuleCategory, RuleSeverity>>;
}

// ============================================================================
// TypeCheckerConfig
// ============================================================================

/**
 * Type checker configuration
 */
export interface TypeCheckerConfig {
  /**
   * Enable type checking
   * @default false
   */
  enabled?: boolean;

  /**
   * Enable strict mode
   * @default false
   */
  strict?: boolean;

  /**
   * Check component props
   * @default true
   */
  checkProps?: boolean;

  /**
   * Check component emits
   * @default true
   */
  checkEmits?: boolean;

  /**
   * Check template bindings
   * @default true
   */
  checkTemplateBindings?: boolean;

  /**
   * Path to tsconfig.json
   * @default auto-detected
   */
  tsconfig?: string;

  /**
   * Path to tsgo binary
   */
  tsgoPath?: string;
}

// ============================================================================
// FormatterConfig
// ============================================================================

/**
 * Formatter configuration
 */
export interface FormatterConfig {
  /**
   * Max line width
   * @default 80
   */
  printWidth?: number;

  /**
   * Indentation width
   * @default 2
   */
  tabWidth?: number;

  /**
   * Use tabs for indentation
   * @default false
   */
  useTabs?: boolean;

  /**
   * Print semicolons
   * @default true
   */
  semi?: boolean;

  /**
   * Use single quotes
   * @default false
   */
  singleQuote?: boolean;

  /**
   * Trailing commas
   * @default 'all'
   */
  trailingComma?: "all" | "none" | "es5";
}

// ============================================================================
// LspConfig
// ============================================================================

/**
 * LSP configuration
 */
export interface LspConfig {
  /**
   * Enable LSP
   * @default true
   */
  enabled?: boolean;

  /**
   * Enable diagnostics
   * @default true
   */
  diagnostics?: boolean;

  /**
   * Enable completions
   * @default true
   */
  completion?: boolean;

  /**
   * Enable hover information
   * @default true
   */
  hover?: boolean;

  /**
   * Enable go-to-definition
   * @default true
   */
  definition?: boolean;

  /**
   * Enable formatting via LSP
   * @default true
   */
  formatting?: boolean;

  /**
   * Enable code actions
   * @default true
   */
  codeActions?: boolean;

  /**
   * Use tsgo for type checking in LSP
   * @default false
   */
  tsgo?: boolean;
}

// ============================================================================
// MuseaConfig
// ============================================================================

/**
 * VRT (Visual Regression Testing) configuration for Musea
 */
export interface MuseaVrtConfig {
  /**
   * Threshold for pixel comparison (0-1)
   * @default 0.1
   */
  threshold?: number;

  /**
   * Output directory for screenshots
   * @default '__musea_snapshots__'
   */
  outDir?: string;

  /**
   * Viewport sizes
   */
  viewports?: Array<{ width: number; height: number; name?: string }>;
}

/**
 * A11y configuration for Musea
 */
export interface MuseaA11yConfig {
  /**
   * Enable a11y checking
   * @default false
   */
  enabled?: boolean;

  /**
   * Axe-core rules to enable/disable
   */
  rules?: Record<string, boolean>;
}

/**
 * Autogen configuration for Musea
 */
export interface MuseaAutogenConfig {
  /**
   * Enable auto-generation of variants
   * @default false
   */
  enabled?: boolean;

  /**
   * Max variants to generate per component
   * @default 10
   */
  maxVariants?: number;
}

/**
 * Musea component gallery configuration
 */
export interface MuseaConfig {
  /**
   * Glob patterns for art files
   * @default ['**\/*.art.vue']
   */
  include?: string[];

  /**
   * Glob patterns to exclude
   * @default ['node_modules/**', 'dist/**']
   */
  exclude?: string[];

  /**
   * Base path for gallery
   * @default '/__musea__'
   */
  basePath?: string;

  /**
   * Enable Storybook compatibility
   * @default false
   */
  storybookCompat?: boolean;

  /**
   * Enable inline art detection in .vue files
   * @default false
   */
  inlineArt?: boolean;

  /**
   * VRT configuration
   */
  vrt?: MuseaVrtConfig;

  /**
   * A11y configuration
   */
  a11y?: MuseaA11yConfig;

  /**
   * Autogen configuration
   */
  autogen?: MuseaAutogenConfig;
}

// ============================================================================
// GlobalTypesConfig
// ============================================================================

/**
 * Global type declaration
 */
export interface GlobalTypeDeclaration {
  /**
   * TypeScript type string
   */
  type: string;

  /**
   * Default value
   */
  defaultValue?: string;
}

/**
 * Global types configuration
 */
export type GlobalTypesConfig = Record<string, GlobalTypeDeclaration | string>;

// ============================================================================
// LoadConfigOptions
// ============================================================================

/**
 * Options for loading vize.config file
 */
export interface LoadConfigOptions {
  /**
   * Config file search mode
   * - 'root': Search only in the specified root directory
   * - 'auto': Search from cwd upward until finding a config file
   * - 'none': Don't load config file
   * @default 'root'
   */
  mode?: "root" | "auto" | "none";

  /**
   * Custom config file path (overrides automatic search)
   */
  configFile?: string;

  /**
   * Config environment for dynamic config resolution
   */
  env?: ConfigEnv;
}
