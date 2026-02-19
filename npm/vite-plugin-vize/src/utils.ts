import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import type { CompiledModule } from "./types.js";
import { type HmrUpdateType, generateHmrCode } from "./hmr.js";

export function generateScopeId(filename: string): string {
  const hash = createHash("sha256").update(filename).digest("hex");
  return hash.slice(0, 8);
}

export function createFilter(
  include?: string | RegExp | (string | RegExp)[],
  exclude?: string | RegExp | (string | RegExp)[],
): (id: string) => boolean {
  const includePatterns = include ? (Array.isArray(include) ? include : [include]) : [/\.vue$/];
  const excludePatterns = exclude
    ? Array.isArray(exclude)
      ? exclude
      : [exclude]
    : [/node_modules/];

  return (id: string) => {
    const matchInclude = includePatterns.some((pattern) =>
      typeof pattern === "string" ? id.includes(pattern) : pattern.test(id),
    );
    const matchExclude = excludePatterns.some((pattern) =>
      typeof pattern === "string" ? id.includes(pattern) : pattern.test(id),
    );
    return matchInclude && !matchExclude;
  };
}

export interface GenerateOutputOptions {
  isProduction: boolean;
  isDev: boolean;
  hmrUpdateType?: HmrUpdateType;
  extractCss?: boolean;
}

export function generateOutput(compiled: CompiledModule, options: GenerateOutputOptions): string {
  const { isProduction, isDev, hmrUpdateType, extractCss } = options;

  let output = compiled.code;

  // Rewrite "export default" to named variable for HMR
  // Use regex to match only line-start "export default" (not inside strings)
  const exportDefaultRegex = /^export default /m;
  const hasExportDefault = exportDefaultRegex.test(output);

  // Check if _sfc_main is already defined (Case 2: non-script-setup SFCs)
  // In this case, the compiler already outputs: const _sfc_main = ...; export default _sfc_main
  const hasSfcMainDefined = /\bconst\s+_sfc_main\s*=/.test(output);

  if (hasExportDefault && !hasSfcMainDefined) {
    output = output.replace(exportDefaultRegex, "const _sfc_main = ");
    // Add __scopeId for scoped CSS support
    if (compiled.hasScoped && compiled.scopeId) {
      output += `\n_sfc_main.__scopeId = "data-v-${compiled.scopeId}";`;
    }
    output += "\nexport default _sfc_main;";
  } else if (hasExportDefault && hasSfcMainDefined) {
    // _sfc_main already defined, just add scopeId if needed
    if (compiled.hasScoped && compiled.scopeId) {
      // Insert scopeId assignment before the export default line
      output = output.replace(
        /^export default _sfc_main/m,
        `_sfc_main.__scopeId = "data-v-${compiled.scopeId}";\nexport default _sfc_main`,
      );
    }
  }

  // Inject CSS (skip in production if extracting)
  if (compiled.css && !(isProduction && extractCss)) {
    const cssCode = JSON.stringify(compiled.css);
    const cssId = JSON.stringify(`vize-style-${compiled.scopeId}`);
    output = `
const __vize_css__ = ${cssCode};
const __vize_css_id__ = ${cssId};
(function() {
  if (typeof document !== 'undefined') {
    let style = document.getElementById(__vize_css_id__);
    if (!style) {
      style = document.createElement('style');
      style.id = __vize_css_id__;
      style.textContent = __vize_css__;
      document.head.appendChild(style);
    } else {
      style.textContent = __vize_css__;
    }
  }
})();
${output}`;
  }

  // Add HMR support in development (skip in production)
  if (!isProduction && isDev && hasExportDefault) {
    output += generateHmrCode(compiled.scopeId, hmrUpdateType ?? "full-reload");
  }

  return output;
}

/**
 * Legacy generateOutput signature for backward compatibility.
 */
export function generateOutputLegacy(
  compiled: CompiledModule,
  isProduction: boolean,
  isDev: boolean,
): string {
  return generateOutput(compiled, { isProduction, isDev });
}

export interface CssAliasRule {
  find: string;
  replacement: string;
}

/**
 * Resolve CSS @import statements by inlining the imported files,
 * then resolve @custom-media definitions within the combined CSS.
 *
 * This is necessary because Vize embeds CSS as a JS string via
 * document.createElement('style'), bypassing Vite's CSS pipeline.
 */
export function resolveCssImports(
  css: string,
  importer: string,
  aliasRules: CssAliasRule[],
  isDev?: boolean,
  devUrlBase?: string,
): string {
  // Collect @custom-media definitions and imported content
  const customMedia = new Map<string, string>();
  const importRegex = /^@import\s+(?:"([^"]+)"|'([^']+)');?\s*$/gm;
  let result = css;

  // Phase 1: Resolve @import — inline imported file contents
  result = result.replace(importRegex, (_match, dqPath?: string, sqPath?: string) => {
    const importPath = dqPath || sqPath;
    if (!importPath) return _match;

    const resolved = resolveCssPath(importPath, importer, aliasRules);
    if (!resolved || !fs.existsSync(resolved)) {
      return _match; // Keep unresolved imports as-is
    }

    try {
      const content = fs.readFileSync(resolved, "utf-8");
      // Parse @custom-media from imported file
      parseCustomMedia(content, customMedia);
      return content;
    } catch {
      return _match;
    }
  });

  // Also parse @custom-media from the main CSS itself
  parseCustomMedia(result, customMedia);

  // Phase 2: Remove @custom-media definitions from output
  result = result.replace(/^@custom-media\s+[^;]+;\s*$/gm, "");

  // Phase 3: Replace @media (--name) with resolved values
  if (customMedia.size > 0) {
    for (const [name, query] of customMedia) {
      // Replace (--name) in @media rules
      const escaped = name.replace(/[-/\\^$*+?.()|[\]{}]/g, "\\$&");
      result = result.replace(new RegExp(`\\(${escaped}\\)`, "g"), query);
    }
  }

  // Phase 4: Resolve url() references with alias prefixes
  if (isDev) {
    result = result.replace(
      /url\(\s*(["']?)([^"')]+)\1\s*\)/g,
      (_match, quote, urlPath) => {
        const trimmed = urlPath.trim();
        // Skip data: URLs, absolute http(s) URLs, and already-resolved paths
        if (
          trimmed.startsWith("data:") ||
          trimmed.startsWith("http://") ||
          trimmed.startsWith("https://") ||
          trimmed.startsWith("/@fs/")
        ) {
          return _match;
        }
        const resolved = resolveCssPath(trimmed, importer, aliasRules);
        if (resolved && fs.existsSync(resolved)) {
          const normalized = resolved.replace(/\\/g, "/");
          // In Nuxt, Vite is mounted under a base path (e.g., /_nuxt/),
          // so /@fs/ URLs must be prefixed with the base to reach Vite's middleware.
          const base = devUrlBase ?? "/";
          const prefix = base.endsWith("/") ? base : base + "/";
          return `url("${prefix}@fs${normalized}")`;
        }
        return _match;
      },
    );
  }

  // Phase 5: Unwrap Vue scoped CSS pseudo-selectors (:deep, :slotted, :global)
  // Vize uses native CSS nesting with scope attribute only on the root element,
  // so :deep(X) is simply X (no scope attribute to remove from child selectors).
  result = result.replace(/:deep\(([^()]*(?:\([^()]*\))*[^()]*)\)/g, "$1");

  // Clean up excessive blank lines
  result = result.replace(/\n{3,}/g, "\n\n");

  return result;
}

function parseCustomMedia(css: string, map: Map<string, string>): void {
  const re = /@custom-media\s+(--[\w-]+)\s+(.+?)\s*;/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(css)) !== null) {
    map.set(m[1], m[2]);
  }
}

function resolveCssPath(
  importPath: string,
  importer: string,
  aliasRules: CssAliasRule[],
): string | null {
  // Try alias resolution
  for (const rule of aliasRules) {
    if (importPath.startsWith(rule.find)) {
      const resolved = importPath.replace(rule.find, rule.replacement);
      return path.resolve(resolved);
    }
  }

  // Relative path
  if (importPath.startsWith(".")) {
    const dir = path.dirname(importer);
    return path.resolve(dir, importPath);
  }

  // Absolute path
  if (path.isAbsolute(importPath)) {
    return importPath;
  }

  return null;
}
