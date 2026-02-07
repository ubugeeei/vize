/**
 * Musea plugin options.
 */
export interface MuseaOptions {
  /**
   * Glob patterns to include Art files.
   * @default ['**\/*.art.vue']
   */
  include?: string[];

  /**
   * Glob patterns to exclude.
   * @default ['node_modules/**', 'dist/**']
   */
  exclude?: string[];

  /**
   * Base path for Musea gallery UI.
   * @default '/__musea__'
   */
  basePath?: string;

  /**
   * Enable Storybook CSF output.
   * @default false
   */
  storybookCompat?: boolean;

  /**
   * Storybook output directory (when storybookCompat is true).
   * @default '.storybook/stories'
   */
  storybookOutDir?: string;

  /**
   * Enable inline <art> blocks in regular .vue SFC files.
   * When enabled, regular .vue files containing <art> blocks will be
   * included in the gallery. Use <Self> to reference the host component.
   * @default false
   */
  inlineArt?: boolean;

  /**
   * VRT (Visual Regression Testing) configuration.
   */
  vrt?: VrtOptions;
}

/**
 * VRT configuration options.
 */
export interface VrtOptions {
  /**
   * Snapshot storage directory.
   * @default '.vize/snapshots'
   */
  snapshotDir?: string;

  /**
   * Pixel difference threshold for comparison.
   * @default 100
   */
  threshold?: number;

  /**
   * Viewports to capture.
   * @default [{ width: 1280, height: 720 }, { width: 375, height: 667 }]
   */
  viewports?: ViewportConfig[];
}

/**
 * Viewport configuration.
 */
export interface ViewportConfig {
  /** Width in pixels */
  width: number;
  /** Height in pixels */
  height: number;
  /** Device scale factor */
  deviceScaleFactor?: number;
  /** Viewport name for identification */
  name?: string;
}

/**
 * Art file metadata.
 */
export interface ArtMetadata {
  title: string;
  description?: string;
  component?: string;
  category?: string;
  tags: string[];
  status: "draft" | "ready" | "deprecated";
  order?: number;
}

/**
 * Art variant definition.
 */
export interface ArtVariant {
  name: string;
  template: string;
  isDefault: boolean;
  skipVrt: boolean;
  args?: Record<string, unknown>;
}

/**
 * Parsed Art file information.
 */
export interface ArtFileInfo {
  /** Absolute file path */
  path: string;
  /** Art metadata */
  metadata: ArtMetadata;
  /** Variant definitions */
  variants: ArtVariant[];
  /** Whether file has script setup */
  hasScriptSetup: boolean;
  /** Whether file has regular script */
  hasScript: boolean;
  /** Number of style blocks */
  styleCount: number;
  /** Whether this art comes from an inline <art> block in a regular .vue file */
  isInline?: boolean;
  /** For inline art: absolute path to the host .vue component file */
  componentPath?: string;
}

/**
 * CSF output from Art transformation.
 */
export interface CsfOutput {
  /** Generated CSF code */
  code: string;
  /** Suggested filename */
  filename: string;
}

// ============================================================================
// Palette / Analysis API types
// ============================================================================

/**
 * Palette API response.
 */
export interface PaletteApiResponse {
  title: string;
  controls: PaletteControl[];
  groups: string[];
  json: string;
  typescript: string;
}

/**
 * Single prop control definition.
 */
export interface PaletteControl {
  name: string;
  control: ControlKind;
  default_value?: unknown;
  description?: string;
  required: boolean;
  options: Array<{ label: string; value: unknown }>;
  range?: { min: number; max: number; step?: number };
  group?: string;
}

/**
 * Supported control kinds for the props panel.
 */
export type ControlKind =
  | "text"
  | "number"
  | "boolean"
  | "range"
  | "select"
  | "radio"
  | "color"
  | "date"
  | "object"
  | "array"
  | "file"
  | "raw";

/**
 * Analysis API response (Props/Emits info).
 */
export interface AnalysisApiResponse {
  props: Array<{
    name: string;
    type: string;
    required: boolean;
    default_value?: unknown;
  }>;
  emits: string[];
}

// ============================================================================
// VRT extended types (aligned with Rust VrtConfig)
// ============================================================================

/**
 * Screenshot capture configuration.
 */
export interface CaptureConfig {
  /** Capture full page vs viewport only */
  fullPage?: boolean;
  /** Wait for network idle before capture */
  waitForNetwork?: boolean;
  /** Additional wait time after load (ms) */
  settleTime?: number;
  /** CSS selector to wait for */
  waitSelector?: string;
  /** Elements to hide before capture (CSS selectors) */
  hideElements?: string[];
  /** Elements to mask before capture (CSS selectors) */
  maskElements?: string[];
}

/**
 * Image comparison configuration.
 */
export interface ComparisonConfig {
  /** Anti-aliasing detection */
  antiAliasing?: boolean;
  /** Alpha channel comparison */
  alpha?: boolean;
  /** Diff image output format */
  diffStyle?: "overlay" | "sideBySide" | "diffOnly" | "animated";
  /** Diff highlight color */
  diffColor?: { r: number; g: number; b: number };
}

/**
 * CI-specific configuration.
 */
export interface CiConfig {
  /** Fail build on any diff */
  failOnDiff?: boolean;
  /** Auto-update baselines on main branch */
  autoUpdateOnMain?: boolean;
  /** Generate JSON report for CI */
  jsonReport?: boolean;
  /** Retry failed tests */
  retries?: number;
}

// ============================================================================
// Accessibility types
// ============================================================================

/**
 * Accessibility testing options.
 */
export interface A11yOptions {
  /** Enable a11y auditing during VRT */
  enabled?: boolean;
  /** axe-core rules to include */
  includeRules?: string[];
  /** axe-core rules to exclude */
  excludeRules?: string[];
  /** WCAG level (A, AA, AAA) */
  level?: "A" | "AA" | "AAA";
}

/**
 * Accessibility audit result.
 */
export interface A11yResult {
  artPath: string;
  variantName: string;
  violations: A11yViolation[];
  passes: number;
  incomplete: number;
}

/**
 * Single accessibility violation.
 */
export interface A11yViolation {
  id: string;
  impact: "minor" | "moderate" | "serious" | "critical";
  description: string;
  helpUrl: string;
  nodes: number;
}
