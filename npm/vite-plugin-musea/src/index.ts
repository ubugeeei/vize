/**
 * Vite plugin for Musea - Component gallery for Vue components.
 *
 * @example
 * ```ts
 * import { defineConfig } from 'vite';
 * import { vize } from '@vizejs/vite-plugin';
 * import { musea } from '@vizejs/vite-plugin-musea';
 *
 * export default defineConfig({
 *   plugins: [vize(), musea()],
 * });
 * ```
 */

import type { Plugin, ViteDevServer, ResolvedConfig } from "vite";
import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import { vizeConfigStore } from "@vizejs/vite-plugin";

import type {
  MuseaOptions,
  MuseaTheme,
  MuseaThemeColors,
  ArtFileInfo,
  ArtMetadata,
  ArtVariant,
  CsfOutput,
  PaletteApiResponse,
  AnalysisApiResponse,
  A11yOptions,
  A11yResult,
  CaptureConfig,
  ComparisonConfig,
  CiConfig,
} from "./types.js";

export type {
  MuseaOptions,
  MuseaTheme,
  MuseaThemeColors,
  ArtFileInfo,
  ArtMetadata,
  ArtVariant,
  CsfOutput,
  VrtOptions,
  ViewportConfig,
  PaletteApiResponse,
  AnalysisApiResponse,
  A11yOptions,
  A11yResult,
  CaptureConfig,
  ComparisonConfig,
  CiConfig,
} from "./types.js";

export {
  MuseaVrtRunner,
  generateVrtReport,
  generateVrtJsonReport,
  type VrtResult,
  type VrtSummary,
} from "./vrt.js";

import {
  processStyleDictionary,
  parseTokens,
  generateTokensHtml,
  generateTokensMarkdown,
  buildTokenMap,
  resolveReferences,
  readRawTokenFile,
  writeRawTokenFile,
  setTokenAtPath,
  deleteTokenAtPath,
  validateSemanticReference,
  findDependentTokens,
  scanTokenUsage,
  type DesignToken,
  type TokenCategory,
  type StyleDictionaryConfig,
  type StyleDictionaryOutput,
  type TokenUsageMap,
} from "./style-dictionary.js";

export {
  processStyleDictionary,
  parseTokens,
  generateTokensHtml,
  generateTokensMarkdown,
  buildTokenMap,
  resolveReferences,
  scanTokenUsage,
  type DesignToken,
  type TokenCategory,
  type StyleDictionaryConfig,
  type StyleDictionaryOutput,
  type TokenUsageMap,
};

export { MuseaA11yRunner, type A11ySummary } from "./a11y.js";

export {
  generateArtFile,
  writeArtFile,
  type AutogenOptions,
  type AutogenOutput,
  type PropDefinition,
  type GeneratedVariant,
} from "./autogen.js";

// Virtual module prefixes
const VIRTUAL_MUSEA_PREFIX = "\0musea:";
const VIRTUAL_GALLERY = "\0musea-gallery";
const VIRTUAL_MANIFEST = "\0musea-manifest";

// Native binding types
interface NativeBinding {
  parseArt: (
    source: string,
    options?: { filename?: string },
  ) => {
    filename: string;
    metadata: {
      title: string;
      description?: string;
      component?: string;
      category?: string;
      tags: string[];
      status: string;
      order?: number;
    };
    variants: Array<{
      name: string;
      template: string;
      isDefault: boolean;
      skipVrt: boolean;
    }>;
    hasScriptSetup: boolean;
    hasScript: boolean;
    styleCount: number;
  };
  artToCsf: (
    source: string,
    options?: { filename?: string },
  ) => {
    code: string;
    filename: string;
  };
  generateArtPalette?: (
    source: string,
    artOptions?: { filename?: string },
    paletteOptions?: { infer_options?: boolean; group_by_type?: boolean },
  ) => {
    title: string;
    controls: Array<{
      name: string;
      control: string;
      default_value?: unknown;
      description?: string;
      required: boolean;
      options: Array<{ label: string; value: unknown }>;
      range?: { min: number; max: number; step?: number };
      group?: string;
    }>;
    groups: string[];
    json: string;
    typescript: string;
  };
  generateArtDoc?: (
    source: string,
    artOptions?: { filename?: string },
    docOptions?: {
      include_source?: boolean;
      include_templates?: boolean;
      include_metadata?: boolean;
    },
  ) => {
    markdown: string;
    filename: string;
    title: string;
    category?: string;
    variant_count: number;
  };
  analyzeSfc?: (
    source: string,
    options?: { filename?: string },
  ) => {
    props: Array<{
      name: string;
      type: string;
      required: boolean;
      default_value?: unknown;
    }>;
    emits: string[];
  };
}

// Lazy-load native binding
let native: NativeBinding | null = null;

function loadNative(): NativeBinding {
  if (native) return native;

  const require = createRequire(import.meta.url);
  try {
    native = require("@vizejs/native") as NativeBinding;
    return native;
  } catch (e) {
    throw new Error(
      `Failed to load @vizejs/native. Make sure it's installed and built:\n${String(e)}`,
    );
  }
}

/**
 * JS-based fallback for SFC analysis when native `analyzeSfc` is not available.
 * Uses regex parsing to extract props and emits from Vue SFC source.
 */
function analyzeSfcFallback(
  source: string,
  _options?: { filename?: string },
): {
  props: Array<{
    name: string;
    type: string;
    required: boolean;
    default_value?: unknown;
  }>;
  emits: string[];
} {
  try {
    const props: Array<{
      name: string;
      type: string;
      required: boolean;
      default_value?: unknown;
    }> = [];
    const emits: string[] = [];

    // Extract the <script setup> block
    const scriptSetupMatch = source.match(/<script\s+[^>]*setup[^>]*>([\s\S]*?)<\/script>/);
    if (!scriptSetupMatch) {
      // Try regular <script> block
      const scriptMatch = source.match(/<script[^>]*>([\s\S]*?)<\/script>/);
      if (!scriptMatch) return { props: [], emits: [] };
    }
    const scriptContent = scriptSetupMatch?.[1] || "";

    // Extract defineProps type parameter
    // Handles: defineProps<{ ... }>()  and  defineProps<{ ... }>
    const propsMatch = scriptContent.match(/defineProps\s*<\s*\{([\s\S]*?)\}>\s*\(/);
    const propsMatch2 = scriptContent.match(/defineProps\s*<\s*\{([\s\S]*?)\}>/);
    const propsBody = propsMatch?.[1] || propsMatch2?.[1];

    if (propsBody) {
      // Parse each prop line: name?: Type;  or  name: Type;
      // Handle multiline JSDoc comments before props
      const lines = propsBody.split("\n");
      let i = 0;
      while (i < lines.length) {
        const line = lines[i].trim();
        // Skip JSDoc comments
        if (line.startsWith("/**") || line.startsWith("*") || line.startsWith("*/")) {
          i++;
          continue;
        }

        // Match prop definition: name?: Type  or  name: Type
        const propMatch = line.match(/^(\w+)(\?)?:\s*(.+?)(?:;?\s*)$/);
        if (propMatch) {
          const name = propMatch[1];
          const optional = !!propMatch[2];
          let type = propMatch[3].replace(/;$/, "").trim();

          // Check for default value in destructured defineProps
          const defaultPattern = new RegExp(`\\b${name}\\s*=\\s*([^,}\\n]+)`);
          const defaultMatch = scriptContent.match(defaultPattern);
          const defaultValue = defaultMatch ? defaultMatch[1].trim() : undefined;

          props.push({
            name,
            type,
            required: !optional && defaultValue === undefined,
            ...(defaultValue !== undefined ? { default_value: defaultValue } : {}),
          });
        }
        i++;
      }
    }

    // Extract defineEmits
    const emitsMatch = scriptContent.match(/defineEmits\s*<\s*\{([\s\S]*?)\}>/);
    if (emitsMatch) {
      const emitsBody = emitsMatch[1];
      const emitRegex = /(\w+)\s*:/g;
      let match;
      while ((match = emitRegex.exec(emitsBody)) !== null) {
        emits.push(match[1]);
      }
    }

    return { props, emits };
  } catch {
    return { props: [], emits: [] };
  }
}

/**
 * Build the theme config object from plugin options for runtime injection.
 */
function buildThemeConfig(theme?: MuseaOptions["theme"]):
  | {
      default: string;
      custom?: Record<string, { base?: "dark" | "light"; colors: Record<string, string> }>;
    }
  | undefined {
  if (!theme) return undefined;

  if (typeof theme === "string") {
    // 'dark' | 'light' | 'system'
    return { default: theme };
  }

  // Single custom theme or array of custom themes
  const themes = Array.isArray(theme) ? theme : [theme];
  const custom: Record<string, { base?: "dark" | "light"; colors: Record<string, string> }> = {};
  for (const t of themes) {
    custom[t.name] = {
      base: t.base,
      colors: t.colors as Record<string, string>,
    };
  }
  return {
    default: themes[0].name,
    custom,
  };
}

/**
 * Create Musea Vite plugin.
 */
export function musea(options: MuseaOptions = {}): Plugin[] {
  let include = options.include ?? ["**/*.art.vue"];
  let exclude = options.exclude ?? ["node_modules/**", "dist/**"];
  let basePath = options.basePath ?? "/__musea__";
  let storybookCompat = options.storybookCompat ?? false;
  const storybookOutDir = options.storybookOutDir ?? ".storybook/stories";
  let inlineArt = options.inlineArt ?? false;
  const tokensPath = options.tokensPath;
  const themeConfig = buildThemeConfig(options.theme);
  const previewCss = options.previewCss ?? [];
  const previewSetup = options.previewSetup;

  let config: ResolvedConfig;
  let server: ViteDevServer | null = null;
  const artFiles = new Map<string, ArtFileInfo>();
  let resolvedPreviewCss: string[] = [];
  let resolvedPreviewSetup: string | null = null;

  // Main plugin
  const mainPlugin: Plugin = {
    name: "vite-plugin-musea",
    enforce: "pre",

    config() {
      // Add Vue alias for runtime template compilation
      // This is needed because variant templates are compiled at runtime
      return {
        resolve: {
          alias: {
            vue: "vue/dist/vue.esm-bundler.js",
          },
        },
      };
    },

    configResolved(resolvedConfig) {
      config = resolvedConfig;

      // Merge musea config from vize.config.ts (plugin args > config file > defaults)
      const vizeConfig = vizeConfigStore.get(resolvedConfig.root);
      if (vizeConfig?.musea) {
        const mc = vizeConfig.musea;
        // Only apply config file values when plugin options were not explicitly set
        if (!options.include && mc.include) include = mc.include;
        if (!options.exclude && mc.exclude) exclude = mc.exclude;
        if (!options.basePath && mc.basePath) basePath = mc.basePath;
        if (options.storybookCompat === undefined && mc.storybookCompat !== undefined)
          storybookCompat = mc.storybookCompat;
        if (options.inlineArt === undefined && mc.inlineArt !== undefined) inlineArt = mc.inlineArt;
      }

      // Resolve previewCss paths to absolute paths
      resolvedPreviewCss = previewCss.map((cssPath) =>
        path.isAbsolute(cssPath) ? cssPath : path.resolve(resolvedConfig.root, cssPath),
      );

      // Resolve previewSetup path
      if (previewSetup) {
        resolvedPreviewSetup = path.isAbsolute(previewSetup)
          ? previewSetup
          : path.resolve(resolvedConfig.root, previewSetup);
      }
    },

    configureServer(devServer) {
      server = devServer;

      // Gallery SPA route - serves built SPA or falls back to inline HTML
      devServer.middlewares.use(basePath, async (req, res, next) => {
        const url = req.url || "/";

        // Serve SPA for gallery routes (not /api/, /preview, /preview-module, /art)
        if (
          url === "/" ||
          url === "/index.html" ||
          url.startsWith("/tokens") ||
          url.startsWith("/component/")
        ) {
          // Try serving built SPA first
          const galleryDistDir = path.resolve(
            path.dirname(new URL(import.meta.url).pathname),
            "gallery",
          );
          const indexHtmlPath = path.join(galleryDistDir, "index.html");

          try {
            await fs.promises.access(indexHtmlPath);
            let html = await fs.promises.readFile(indexHtmlPath, "utf-8");
            // Inject basePath and theme config for runtime use
            const themeScript = themeConfig
              ? `window.__MUSEA_THEME_CONFIG__=${JSON.stringify(themeConfig)};`
              : "";
            html = html.replace(
              "</head>",
              `<script>window.__MUSEA_BASE_PATH__='${basePath}';${themeScript}</script></head>`,
            );
            // NOTE: Don't use transformIndexHtml here - it rewrites asset paths
            // (e.g. prepending /_nuxt/) which breaks the pre-built gallery assets.
            res.setHeader("Content-Type", "text/html");
            res.end(html);
            return;
          } catch {
            // Fall back to inline gallery HTML
            const html = generateGalleryHtml(basePath, themeConfig);
            res.setHeader("Content-Type", "text/html");
            res.end(html);
            return;
          }
        }
        // Serve gallery static assets (JS, CSS) from built SPA
        if (url.startsWith("/assets/")) {
          const galleryDistDir = path.resolve(
            path.dirname(new URL(import.meta.url).pathname),
            "gallery",
          );
          const filePath = path.join(galleryDistDir, url);
          try {
            const stat = await fs.promises.stat(filePath);
            if (stat.isFile()) {
              const content = await fs.promises.readFile(filePath);
              const ext = path.extname(filePath);
              const mimeTypes: Record<string, string> = {
                ".js": "application/javascript",
                ".css": "text/css",
                ".svg": "image/svg+xml",
                ".png": "image/png",
                ".ico": "image/x-icon",
                ".woff2": "font/woff2",
                ".woff": "font/woff",
              };
              res.setHeader("Content-Type", mimeTypes[ext] || "application/octet-stream");
              res.setHeader("Cache-Control", "public, max-age=31536000, immutable");
              res.end(content);
              return;
            }
          } catch {
            // File not found, fall through
          }
        }

        next();
      });

      // Preview module route - serves the JavaScript module for a specific variant
      devServer.middlewares.use(`${basePath}/preview-module`, async (req, res, _next) => {
        const url = new URL(req.url || "", `http://localhost`);
        const artPath = url.searchParams.get("art");
        const variantName = url.searchParams.get("variant");

        if (!artPath || !variantName) {
          res.statusCode = 400;
          res.end("Missing art or variant parameter");
          return;
        }

        const art = artFiles.get(artPath);
        if (!art) {
          res.statusCode = 404;
          res.end("Art not found");
          return;
        }

        const variant = art.variants.find((v) => v.name === variantName);
        if (!variant) {
          res.statusCode = 404;
          res.end("Variant not found");
          return;
        }

        const variantComponentName = toPascalCase(variant.name);
        const moduleCode = generatePreviewModule(
          art,
          variantComponentName,
          variant.name,
          resolvedPreviewCss,
          resolvedPreviewSetup,
        );

        // Transform the module through Vite to resolve imports
        try {
          const result = await devServer.transformRequest(
            `virtual:musea-preview:${artPath}:${variantName}`,
          );
          if (result) {
            res.setHeader("Content-Type", "application/javascript");
            res.setHeader("Cache-Control", "no-cache");
            res.end(result.code);
            return;
          }
        } catch {
          // Fall through to manual response
        }

        // Fallback: serve the module directly (imports won't be resolved)
        res.setHeader("Content-Type", "application/javascript");
        res.setHeader("Cache-Control", "no-cache");
        res.end(moduleCode);
      });

      // VRT preview route - renders a single variant for screenshot
      devServer.middlewares.use(`${basePath}/preview`, async (req, res, _next) => {
        const url = new URL(req.url || "", `http://localhost`);
        const artPath = url.searchParams.get("art");
        const variantName = url.searchParams.get("variant");

        if (!artPath || !variantName) {
          res.statusCode = 400;
          res.end("Missing art or variant parameter");
          return;
        }

        const art = artFiles.get(artPath);
        if (!art) {
          res.statusCode = 404;
          res.end("Art not found");
          return;
        }

        const variant = art.variants.find((v) => v.name === variantName);
        if (!variant) {
          res.statusCode = 404;
          res.end("Variant not found");
          return;
        }

        const html = generatePreviewHtml(art, variant, basePath, config.base);
        // NOTE: Don't use transformIndexHtml here - it creates html-proxy modules
        // for inline scripts, but the URL contains query params which causes
        // malformed proxy URLs (double '?'). Instead, /@vite/client is injected
        // directly in generatePreviewHtml for HMR support.
        res.setHeader("Content-Type", "text/html");
        res.end(html);
      });

      // Art module route - serves transformed art file as ES module
      devServer.middlewares.use(`${basePath}/art`, async (req, res, next) => {
        const url = new URL(req.url || "", "http://localhost");
        const artPath = decodeURIComponent(url.pathname.slice(1)); // Remove leading /

        if (!artPath) {
          next();
          return;
        }

        const art = artFiles.get(artPath);
        if (!art) {
          res.statusCode = 404;
          res.end("Art not found: " + artPath);
          return;
        }

        // Transform through Vite for proper imports
        try {
          const virtualId = `virtual:musea-art:${artPath}`;
          const result = await devServer.transformRequest(virtualId);
          if (result) {
            res.setHeader("Content-Type", "application/javascript");
            res.setHeader("Cache-Control", "no-cache");
            res.end(result.code);
          } else {
            // Fallback: generate and serve the module directly
            const moduleCode = generateArtModule(art, artPath);
            res.setHeader("Content-Type", "application/javascript");
            res.end(moduleCode);
          }
        } catch (err) {
          console.error("[musea] Failed to transform art module:", err);
          // Fallback if transform fails
          const moduleCode = generateArtModule(art, artPath);
          res.setHeader("Content-Type", "application/javascript");
          res.end(moduleCode);
        }
      });

      // API endpoints
      devServer.middlewares.use(`${basePath}/api`, async (req, res, next) => {
        const sendJson = (data: unknown, status = 200) => {
          res.statusCode = status;
          res.setHeader("Content-Type", "application/json");
          res.end(JSON.stringify(data));
        };

        const sendError = (message: string, status = 500) => {
          sendJson({ error: message }, status);
        };

        // GET /api/arts - List all arts
        if (req.url === "/arts" && req.method === "GET") {
          sendJson(Array.from(artFiles.values()));
          return;
        }

        // GET /api/tokens/usage - Get token usage across art files
        if (req.url === "/tokens/usage" && req.method === "GET") {
          if (!tokensPath) {
            sendJson({});
            return;
          }

          try {
            const absoluteTokensPath = path.resolve(config.root, tokensPath);
            const categories = await parseTokens(absoluteTokensPath);
            const tokenMap = buildTokenMap(categories);
            resolveReferences(categories, tokenMap);
            const resolvedTokenMap = buildTokenMap(categories);
            const usage: TokenUsageMap = scanTokenUsage(artFiles, resolvedTokenMap);
            sendJson(usage);
          } catch (e) {
            console.error("[musea] Failed to scan token usage:", e);
            sendJson({});
          }
          return;
        }

        // GET /api/tokens - Get design tokens
        if (req.url === "/tokens" && req.method === "GET") {
          if (!tokensPath) {
            sendJson({
              categories: [],
              tokenMap: {},
              meta: {
                filePath: "",
                tokenCount: 0,
                primitiveCount: 0,
                semanticCount: 0,
              },
            });
            return;
          }

          try {
            const absoluteTokensPath = path.resolve(config.root, tokensPath);
            const categories = await parseTokens(absoluteTokensPath);
            const tokenMap = buildTokenMap(categories);
            resolveReferences(categories, tokenMap);
            // Rebuild tokenMap after resolving to include resolved values
            const resolvedTokenMap = buildTokenMap(categories);
            let primitiveCount = 0;
            let semanticCount = 0;
            for (const token of Object.values(resolvedTokenMap)) {
              if (token.$tier === "semantic") semanticCount++;
              else primitiveCount++;
            }
            sendJson({
              categories,
              tokenMap: resolvedTokenMap,
              meta: {
                filePath: absoluteTokensPath,
                tokenCount: Object.keys(resolvedTokenMap).length,
                primitiveCount,
                semanticCount,
              },
            });
          } catch (e) {
            console.error("[musea] Failed to load tokens:", e);
            sendJson({ categories: [], tokenMap: {}, error: String(e) });
          }
          return;
        }

        // POST /api/tokens - Create a new token
        if (req.url === "/tokens" && req.method === "POST") {
          if (!tokensPath) {
            sendError("No tokens path configured", 400);
            return;
          }
          let body = "";
          req.on("data", (chunk) => {
            body += chunk;
          });
          req.on("end", async () => {
            try {
              const { path: dotPath, token } = JSON.parse(body) as {
                path: string;
                token: Omit<DesignToken, "$resolvedValue">;
              };
              if (!dotPath || !token || token.value === undefined) {
                sendError("Missing required fields: path, token.value", 400);
                return;
              }
              const absoluteTokensPath = path.resolve(config.root, tokensPath);
              const rawData = await readRawTokenFile(absoluteTokensPath);

              // Check path uniqueness
              const currentCategories = await parseTokens(absoluteTokensPath);
              const currentMap = buildTokenMap(currentCategories);
              if (currentMap[dotPath]) {
                sendError(`Token already exists at path "${dotPath}"`, 409);
                return;
              }

              // Validate semantic reference
              if (token.$reference) {
                const validation = validateSemanticReference(currentMap, token.$reference, dotPath);
                if (!validation.valid) {
                  sendError(validation.error!, 400);
                  return;
                }
                token.value = `{${token.$reference}}`;
                token.$tier = "semantic";
              }

              setTokenAtPath(rawData, dotPath, token);
              await writeRawTokenFile(absoluteTokensPath, rawData);

              // Re-parse and return updated data
              const categories = await parseTokens(absoluteTokensPath);
              const tokenMap = buildTokenMap(categories);
              resolveReferences(categories, tokenMap);
              const resolvedTokenMap = buildTokenMap(categories);
              sendJson({ categories, tokenMap: resolvedTokenMap }, 201);
            } catch (e) {
              sendError(e instanceof Error ? e.message : String(e));
            }
          });
          return;
        }

        // PUT /api/tokens - Update an existing token
        if (req.url === "/tokens" && req.method === "PUT") {
          if (!tokensPath) {
            sendError("No tokens path configured", 400);
            return;
          }
          let body = "";
          req.on("data", (chunk) => {
            body += chunk;
          });
          req.on("end", async () => {
            try {
              const { path: dotPath, token } = JSON.parse(body) as {
                path: string;
                token: Omit<DesignToken, "$resolvedValue">;
              };
              if (!dotPath || !token || token.value === undefined) {
                sendError("Missing required fields: path, token.value", 400);
                return;
              }
              const absoluteTokensPath = path.resolve(config.root, tokensPath);

              // Validate semantic reference
              if (token.$reference) {
                const currentCategories = await parseTokens(absoluteTokensPath);
                const currentMap = buildTokenMap(currentCategories);
                const validation = validateSemanticReference(currentMap, token.$reference, dotPath);
                if (!validation.valid) {
                  sendError(validation.error!, 400);
                  return;
                }
                token.value = `{${token.$reference}}`;
                token.$tier = "semantic";
              }

              const rawData = await readRawTokenFile(absoluteTokensPath);
              setTokenAtPath(rawData, dotPath, token);
              await writeRawTokenFile(absoluteTokensPath, rawData);

              const categories = await parseTokens(absoluteTokensPath);
              const tokenMap = buildTokenMap(categories);
              resolveReferences(categories, tokenMap);
              const resolvedTokenMap = buildTokenMap(categories);
              sendJson({ categories, tokenMap: resolvedTokenMap });
            } catch (e) {
              sendError(e instanceof Error ? e.message : String(e));
            }
          });
          return;
        }

        // DELETE /api/tokens - Delete a token
        if (req.url === "/tokens" && req.method === "DELETE") {
          if (!tokensPath) {
            sendError("No tokens path configured", 400);
            return;
          }
          let body = "";
          req.on("data", (chunk) => {
            body += chunk;
          });
          req.on("end", async () => {
            try {
              const { path: dotPath } = JSON.parse(body) as { path: string };
              if (!dotPath) {
                sendError("Missing required field: path", 400);
                return;
              }
              const absoluteTokensPath = path.resolve(config.root, tokensPath);

              // Check for dependent tokens
              const currentCategories = await parseTokens(absoluteTokensPath);
              const currentMap = buildTokenMap(currentCategories);
              const dependents = findDependentTokens(currentMap, dotPath);

              const rawData = await readRawTokenFile(absoluteTokensPath);
              const deleted = deleteTokenAtPath(rawData, dotPath);
              if (!deleted) {
                sendError(`Token not found at path "${dotPath}"`, 404);
                return;
              }
              await writeRawTokenFile(absoluteTokensPath, rawData);

              const categories = await parseTokens(absoluteTokensPath);
              const tokenMap = buildTokenMap(categories);
              resolveReferences(categories, tokenMap);
              const resolvedTokenMap = buildTokenMap(categories);
              sendJson({
                categories,
                tokenMap: resolvedTokenMap,
                dependentsWarning: dependents.length > 0 ? dependents : undefined,
              });
            } catch (e) {
              sendError(e instanceof Error ? e.message : String(e));
            }
          });
          return;
        }

        // PUT /api/arts/:path/source - Update art file source
        if (req.url?.startsWith("/arts/") && req.method === "PUT") {
          const rest = req.url.slice(6);
          const sourceMatch = rest.match(/^(.+)\/source$/);
          if (sourceMatch) {
            const artPath = decodeURIComponent(sourceMatch[1]);
            const art = artFiles.get(artPath);
            if (!art) {
              sendError("Art not found", 404);
              return;
            }

            let body = "";
            req.on("data", (chunk: string) => {
              body += chunk;
            });
            req.on("end", async () => {
              try {
                const { source } = JSON.parse(body) as { source: string };
                if (typeof source !== "string") {
                  sendError("Missing required field: source", 400);
                  return;
                }
                await fs.promises.writeFile(artPath, source, "utf-8");
                await processArtFile(artPath);
                sendJson({ success: true });
              } catch (e) {
                sendError(e instanceof Error ? e.message : String(e));
              }
            });
            return;
          }
          next();
          return;
        }

        // Arts sub-routes: /api/arts/:encodedPath/...
        if (req.url?.startsWith("/arts/") && req.method === "GET") {
          const rest = req.url.slice(6); // after "/arts/"

          // Check for sub-resource patterns
          const sourceMatch = rest.match(/^(.+)\/source$/);
          const paletteMatch = rest.match(/^(.+)\/palette$/);
          const analysisMatch = rest.match(/^(.+)\/analysis$/);
          const docsMatch = rest.match(/^(.+)\/docs$/);
          const a11yMatch = rest.match(/^(.+)\/variants\/([^/]+)\/a11y$/);

          if (sourceMatch) {
            // GET /api/arts/:path/source
            const artPath = decodeURIComponent(sourceMatch[1]);
            const art = artFiles.get(artPath);
            if (!art) {
              sendError("Art not found", 404);
              return;
            }

            try {
              const source = await fs.promises.readFile(artPath, "utf-8");
              sendJson({ source, path: artPath });
            } catch (e) {
              sendError(e instanceof Error ? e.message : String(e));
            }
            return;
          }

          if (paletteMatch) {
            // GET /api/arts/:path/palette
            const artPath = decodeURIComponent(paletteMatch[1]);
            const art = artFiles.get(artPath);
            if (!art) {
              sendError("Art not found", 404);
              return;
            }

            try {
              const source = await fs.promises.readFile(artPath, "utf-8");
              const binding = loadNative();
              let palette: {
                title: string;
                controls: Array<{
                  name: string;
                  control: string;
                  default_value?: unknown;
                  description?: string;
                  required: boolean;
                  options: Array<{ label: string; value: unknown }>;
                  range?: { min: number; max: number; step?: number };
                  group?: string;
                }>;
                groups: string[];
                json: string;
                typescript: string;
              };
              if (binding.generateArtPalette) {
                palette = binding.generateArtPalette(source, {
                  filename: artPath,
                });
              } else {
                palette = {
                  title: art.metadata.title,
                  controls: [],
                  groups: [],
                  json: "{}",
                  typescript: "",
                };
              }

              // If the native palette returned no controls, try JS-based SFC analysis
              if (palette.controls.length === 0 && art.metadata.component) {
                const resolvedComponentPath = path.isAbsolute(art.metadata.component)
                  ? art.metadata.component
                  : path.resolve(path.dirname(artPath), art.metadata.component);
                try {
                  const componentSource = await fs.promises.readFile(
                    resolvedComponentPath,
                    "utf-8",
                  );
                  const analysis = binding.analyzeSfc
                    ? binding.analyzeSfc(componentSource, {
                        filename: resolvedComponentPath,
                      })
                    : analyzeSfcFallback(componentSource, {
                        filename: resolvedComponentPath,
                      });

                  if (analysis.props.length > 0) {
                    palette.controls = analysis.props.map((prop) => {
                      let control = "text";
                      if (prop.type === "boolean") control = "boolean";
                      else if (prop.type === "number") control = "number";
                      else if (prop.type.includes("|") && !prop.type.includes("=>")) {
                        // Union type like "button" | "submit" | "reset" -> select
                        control = "select";
                      }

                      const options: Array<{ label: string; value: unknown }> = [];
                      if (control === "select") {
                        // Extract union type options
                        const optionMatches = prop.type.match(/"([^"]+)"/g);
                        if (optionMatches) {
                          for (const opt of optionMatches) {
                            const val = opt.replace(/"/g, "");
                            options.push({ label: val, value: val });
                          }
                        }
                      }

                      return {
                        name: prop.name,
                        control,
                        default_value:
                          prop.default_value !== undefined
                            ? prop.default_value === "true"
                              ? true
                              : prop.default_value === "false"
                                ? false
                                : typeof prop.default_value === "string" &&
                                    prop.default_value.startsWith('"')
                                  ? prop.default_value.replace(/^"|"$/g, "")
                                  : prop.default_value
                            : undefined,
                        description: undefined,
                        required: prop.required,
                        options,
                        range: undefined,
                        group: undefined,
                      };
                    });

                    // Regenerate json and typescript
                    palette.json = JSON.stringify(
                      { title: palette.title, controls: palette.controls },
                      null,
                      2,
                    );
                    palette.typescript = `export interface ${palette.title}Props {\n${palette.controls
                      .map(
                        (c) =>
                          `  ${c.name}${c.required ? "" : "?"}: ${
                            c.control === "boolean"
                              ? "boolean"
                              : c.control === "number"
                                ? "number"
                                : c.control === "select"
                                  ? c.options.map((o) => `"${String(o.value)}"`).join(" | ")
                                  : "string"
                          };`,
                      )
                      .join("\n")}\n}\n`;
                  }
                } catch {
                  // Ignore errors reading component file
                }
              }

              sendJson(palette);
            } catch (e) {
              sendError(e instanceof Error ? e.message : String(e));
            }
            return;
          }

          if (analysisMatch) {
            // GET /api/arts/:path/analysis
            const artPath = decodeURIComponent(analysisMatch[1]);
            const art = artFiles.get(artPath);
            if (!art) {
              sendError("Art not found", 404);
              return;
            }

            try {
              // Determine the component file path: inline art uses the file itself, .art.vue uses the component attribute
              const resolvedComponentPath =
                art.isInline && art.componentPath
                  ? art.componentPath
                  : art.metadata.component
                    ? path.isAbsolute(art.metadata.component)
                      ? art.metadata.component
                      : path.resolve(path.dirname(artPath), art.metadata.component)
                    : null;

              if (resolvedComponentPath) {
                const source = await fs.promises.readFile(resolvedComponentPath, "utf-8");
                const binding = loadNative();
                if (binding.analyzeSfc) {
                  const analysis = binding.analyzeSfc(source, {
                    filename: resolvedComponentPath,
                  });
                  sendJson(analysis);
                } else {
                  const analysis = analyzeSfcFallback(source, {
                    filename: resolvedComponentPath,
                  });
                  sendJson(analysis);
                }
              } else {
                sendJson({ props: [], emits: [] });
              }
            } catch (e) {
              sendError(e instanceof Error ? e.message : String(e));
            }
            return;
          }

          if (docsMatch) {
            // GET /api/arts/:path/docs
            const artPath = decodeURIComponent(docsMatch[1]);
            const art = artFiles.get(artPath);
            if (!art) {
              sendError("Art not found", 404);
              return;
            }

            try {
              const source = await fs.promises.readFile(artPath, "utf-8");
              const binding = loadNative();
              if (binding.generateArtDoc) {
                const doc = binding.generateArtDoc(source, {
                  filename: artPath,
                });
                // Replace Self with component name and format indentation
                let markdown = doc.markdown || "";
                const componentName = art.metadata.title || "Component";
                markdown = markdown
                  .replace(/<Self(\s|>|\/)/g, `<${componentName}$1`)
                  .replace(/<\/Self>/g, `</${componentName}>`);
                // Fix indentation in code blocks
                markdown = markdown.replace(/```(\w*)\n([\s\S]*?)```/g, (_match, lang, code) => {
                  const lines = code.split("\n");
                  // Find minimum indentation (excluding empty lines)
                  let minIndent = Infinity;
                  for (const line of lines) {
                    if (line.trim()) {
                      const indent = line.match(/^(\s*)/)?.[1].length || 0;
                      minIndent = Math.min(minIndent, indent);
                    }
                  }
                  if (minIndent === Infinity) minIndent = 0;
                  // Remove minimum indentation from all lines
                  const formatted = lines.map((line) => line.slice(minIndent)).join("\n");
                  return "```" + lang + "\n" + formatted + "```";
                });
                sendJson({ ...doc, markdown });
              } else {
                sendJson({
                  markdown: "",
                  title: art.metadata.title,
                  variant_count: art.variants.length,
                });
              }
            } catch (e) {
              sendError(e instanceof Error ? e.message : String(e));
            }
            return;
          }

          if (a11yMatch) {
            // GET /api/arts/:path/variants/:name/a11y
            const artPath = decodeURIComponent(a11yMatch[1]);
            const _variantName = decodeURIComponent(a11yMatch[2]);
            const art = artFiles.get(artPath);
            if (!art) {
              sendError("Art not found", 404);
              return;
            }

            // Return empty a11y results (populated after VRT --a11y run)
            sendJson({ violations: [], passes: 0, incomplete: 0 });
            return;
          }

          // GET /api/arts/:path - Get single art (no sub-resource)
          const artPath = decodeURIComponent(rest);
          const art = artFiles.get(artPath);
          if (art) {
            sendJson(art);
          } else {
            sendError("Art not found", 404);
          }
          return;
        }

        // POST /api/preview-with-props
        if (req.url === "/preview-with-props" && req.method === "POST") {
          let body = "";
          req.on("data", (chunk) => {
            body += chunk;
          });
          req.on("end", () => {
            try {
              const { artPath: reqArtPath, variantName, props: propsOverride } = JSON.parse(body);
              const art = artFiles.get(reqArtPath);
              if (!art) {
                sendError("Art not found", 404);
                return;
              }

              const variant = art.variants.find((v) => v.name === variantName);
              if (!variant) {
                sendError("Variant not found", 404);
                return;
              }

              // Generate preview module with props override
              const variantComponentName = toPascalCase(variant.name);
              const moduleCode = generatePreviewModuleWithProps(
                art,
                variantComponentName,
                variant.name,
                propsOverride,
                resolvedPreviewCss,
                resolvedPreviewSetup,
              );
              res.setHeader("Content-Type", "application/javascript");
              res.end(moduleCode);
            } catch (e) {
              sendError(e instanceof Error ? e.message : String(e));
            }
          });
          return;
        }

        // POST /api/generate
        if (req.url === "/generate" && req.method === "POST") {
          let body = "";
          req.on("data", (chunk) => {
            body += chunk;
          });
          req.on("end", async () => {
            try {
              const { componentPath: reqComponentPath, options: autogenOptions } = JSON.parse(body);
              const { generateArtFile: genArt } = await import("./autogen.js");
              const result = await genArt(reqComponentPath, autogenOptions);
              sendJson({
                generated: true,
                componentName: result.componentName,
                variants: result.variants,
                artFileContent: result.artFileContent,
              });
            } catch (e) {
              sendError(e instanceof Error ? e.message : String(e));
            }
          });
          return;
        }

        // POST /api/run-vrt - Run VRT for a specific art file or all
        if (req.url === "/run-vrt" && req.method === "POST") {
          let body = "";
          req.on("data", (chunk) => {
            body += chunk;
          });
          req.on("end", async () => {
            try {
              const { artPath, updateSnapshots } = JSON.parse(body);
              const { MuseaVrtRunner } = await import("./vrt.js");

              const runner = new MuseaVrtRunner({
                snapshotDir: path.resolve(config.root, ".vize/snapshots"),
              });

              const port = devServer.config.server.port || 5173;
              const baseUrl = `http://localhost:${port}`;

              // Get arts to test
              let artsToTest = Array.from(artFiles.values());
              if (artPath) {
                artsToTest = artsToTest.filter((a) => a.path === artPath);
              }

              await runner.start();
              const results = await runner.runTests(artsToTest, baseUrl, {
                updateSnapshots,
              });
              const summary = runner.getSummary(results);
              await runner.stop();

              sendJson({
                success: true,
                summary,
                results: results.map((r) => ({
                  artPath: r.artPath,
                  variantName: r.variantName,
                  viewport: r.viewport.name,
                  passed: r.passed,
                  isNew: r.isNew,
                  diffPercentage: r.diffPercentage,
                  error: r.error,
                })),
              });
            } catch (e) {
              sendError(e instanceof Error ? e.message : String(e));
            }
          });
          return;
        }

        next();
      });

      // Watch for Art file changes
      devServer.watcher.on("change", async (file) => {
        if (file.endsWith(".art.vue") && shouldProcess(file, include, exclude, config.root)) {
          await processArtFile(file);
          console.log(`[musea] Reloaded: ${path.relative(config.root, file)}`);
        }
        // Inline art: re-check .vue files on change
        if (inlineArt && file.endsWith(".vue") && !file.endsWith(".art.vue")) {
          const hadArt = artFiles.has(file);
          const source = await fs.promises.readFile(file, "utf-8");
          if (source.includes("<art")) {
            await processArtFile(file);
            console.log(`[musea] Reloaded inline art: ${path.relative(config.root, file)}`);
          } else if (hadArt) {
            artFiles.delete(file);
            console.log(`[musea] Removed inline art: ${path.relative(config.root, file)}`);
          }
        }
      });

      devServer.watcher.on("add", async (file) => {
        if (file.endsWith(".art.vue") && shouldProcess(file, include, exclude, config.root)) {
          await processArtFile(file);
          console.log(`[musea] Added: ${path.relative(config.root, file)}`);
        }
        // Inline art: check new .vue files
        if (inlineArt && file.endsWith(".vue") && !file.endsWith(".art.vue")) {
          const source = await fs.promises.readFile(file, "utf-8");
          if (source.includes("<art")) {
            await processArtFile(file);
            console.log(`[musea] Added inline art: ${path.relative(config.root, file)}`);
          }
        }
      });

      devServer.watcher.on("unlink", (file) => {
        if (artFiles.has(file)) {
          artFiles.delete(file);
          console.log(`[musea] Removed: ${path.relative(config.root, file)}`);
        }
      });

      // Print Musea gallery URL after server starts
      return () => {
        devServer.httpServer?.once("listening", () => {
          const address = devServer.httpServer?.address();
          if (address && typeof address === "object") {
            const protocol = devServer.config.server.https ? "https" : "http";
            const rawHost = address.address;
            // Normalize IPv6/IPv4 localhost addresses to "localhost"
            const host =
              rawHost === "::" ||
              rawHost === "::1" ||
              rawHost === "0.0.0.0" ||
              rawHost === "127.0.0.1"
                ? "localhost"
                : rawHost;
            const port = address.port;
            const url = `${protocol}://${host}:${port}${basePath}`;

            console.log();
            console.log(`  \x1b[36m➜\x1b[0m  \x1b[1mMusea Gallery:\x1b[0m \x1b[36m${url}\x1b[0m`);
          }
        });
      };
    },

    async buildStart() {
      // Scan for Art files
      console.log(`[musea] config.root: ${config.root}, include: ${JSON.stringify(include)}`);
      const files = await scanArtFiles(config.root, include, exclude, inlineArt);

      console.log(`[musea] Found ${files.length} art files`);

      for (const file of files) {
        await processArtFile(file);
      }

      // Generate Storybook CSF if enabled
      if (storybookCompat) {
        await generateStorybookFiles(artFiles, config.root, storybookOutDir);
      }
    },

    resolveId(id) {
      if (id === VIRTUAL_GALLERY) {
        return VIRTUAL_GALLERY;
      }
      if (id === VIRTUAL_MANIFEST) {
        return VIRTUAL_MANIFEST;
      }
      // Handle virtual:musea-preview: prefix for preview modules
      if (id.startsWith("virtual:musea-preview:")) {
        return "\0musea-preview:" + id.slice("virtual:musea-preview:".length);
      }
      // Handle virtual:musea-art: prefix for preview modules
      // Append ?musea-virtual to prevent other plugins (e.g. unplugin-vue-i18n)
      // from treating .vue-ending virtual IDs as Vue SFC files
      if (id.startsWith("virtual:musea-art:")) {
        const artPath = id.slice("virtual:musea-art:".length);
        if (artFiles.has(artPath)) {
          return "\0musea-art:" + artPath + "?musea-virtual";
        }
      }
      if (id.endsWith(".art.vue")) {
        const resolved = path.resolve(config.root, id);
        if (artFiles.has(resolved)) {
          return VIRTUAL_MUSEA_PREFIX + resolved + "?musea-virtual";
        }
      }
      // Inline art: resolve .vue files that have <art> blocks
      if (inlineArt && id.endsWith(".vue") && !id.endsWith(".art.vue")) {
        const resolved = path.resolve(config.root, id);
        if (artFiles.has(resolved)) {
          return VIRTUAL_MUSEA_PREFIX + resolved + "?musea-virtual";
        }
      }
      return null;
    },

    load(id) {
      if (id === VIRTUAL_GALLERY) {
        return generateGalleryModule(basePath);
      }
      if (id === VIRTUAL_MANIFEST) {
        return generateManifestModule(artFiles);
      }
      // Handle \0musea-preview: prefix for preview modules
      if (id.startsWith("\0musea-preview:")) {
        const rest = id.slice("\0musea-preview:".length);
        const lastColonIndex = rest.lastIndexOf(":");
        if (lastColonIndex !== -1) {
          const artPath = rest.slice(0, lastColonIndex);
          const variantName = rest.slice(lastColonIndex + 1);
          const art = artFiles.get(artPath);
          if (art) {
            const variantComponentName = toPascalCase(variantName);
            return generatePreviewModule(
              art,
              variantComponentName,
              variantName,
              resolvedPreviewCss,
              resolvedPreviewSetup,
            );
          }
        }
      }
      // Handle \0musea-art: prefix for preview modules
      if (id.startsWith("\0musea-art:")) {
        const artPath = id.slice("\0musea-art:".length).replace(/\?musea-virtual$/, "");
        const art = artFiles.get(artPath);
        if (art) {
          return generateArtModule(art, artPath);
        }
      }
      if (id.startsWith(VIRTUAL_MUSEA_PREFIX)) {
        const realPath = id.slice(VIRTUAL_MUSEA_PREFIX.length).replace(/\?musea-virtual$/, "");
        const art = artFiles.get(realPath);
        if (art) {
          return generateArtModule(art, realPath);
        }
      }
      return null;
    },

    async handleHotUpdate(ctx) {
      const { file } = ctx;
      if (file.endsWith(".art.vue") && artFiles.has(file)) {
        await processArtFile(file);

        // Invalidate virtual modules
        const virtualId = VIRTUAL_MUSEA_PREFIX + file + "?musea-virtual";
        const modules = server?.moduleGraph.getModulesByFile(virtualId);
        if (modules) {
          return [...modules];
        }
      }

      // Inline art: HMR for .vue files with <art> blocks
      if (inlineArt && file.endsWith(".vue") && !file.endsWith(".art.vue") && artFiles.has(file)) {
        await processArtFile(file);

        const virtualId = VIRTUAL_MUSEA_PREFIX + file;
        const modules = server?.moduleGraph.getModulesByFile(virtualId);
        if (modules) {
          return [...modules];
        }
      }

      return undefined;
    },
  };

  // Helper functions scoped to this plugin instance

  async function processArtFile(filePath: string): Promise<void> {
    try {
      const source = await fs.promises.readFile(filePath, "utf-8");
      const binding = loadNative();
      const parsed = binding.parseArt(source, { filename: filePath });

      // Skip files with no variants (e.g. .vue files without <art> block)
      if (!parsed.variants || parsed.variants.length === 0) return;

      const isInline = !filePath.endsWith(".art.vue");

      const info: ArtFileInfo = {
        path: filePath,
        metadata: {
          title: parsed.metadata.title || (isInline ? path.basename(filePath, ".vue") : ""),
          description: parsed.metadata.description,
          component: isInline ? undefined : parsed.metadata.component,
          category: parsed.metadata.category,
          tags: parsed.metadata.tags,
          status: parsed.metadata.status as "draft" | "ready" | "deprecated",
          order: parsed.metadata.order,
        },
        variants: parsed.variants.map((v) => ({
          name: v.name,
          template: v.template,
          isDefault: v.isDefault,
          skipVrt: v.skipVrt,
        })),
        hasScriptSetup: parsed.hasScriptSetup,
        scriptSetupContent: parsed.hasScriptSetup ? extractScriptSetupContent(source) : undefined,
        hasScript: parsed.hasScript,
        styleCount: parsed.styleCount,
        isInline,
        componentPath: isInline ? filePath : undefined,
      };

      artFiles.set(filePath, info);
    } catch (e) {
      console.error(`[musea] Failed to process ${filePath}:`, e);
    }
  }

  return [mainPlugin];
}

// Utility functions

function shouldProcess(file: string, include: string[], exclude: string[], root: string): boolean {
  const relative = path.relative(root, file);

  // Check exclude patterns
  for (const pattern of exclude) {
    if (matchGlob(relative, pattern)) {
      return false;
    }
  }

  // Check include patterns
  for (const pattern of include) {
    if (matchGlob(relative, pattern)) {
      return true;
    }
  }

  return false;
}

function matchGlob(filepath: string, pattern: string): boolean {
  // Simple glob matching (supports * and **)
  // Use placeholder for ** to avoid * replacement interfering
  const PLACEHOLDER = "<<GLOBSTAR>>";
  const regex = pattern
    .replaceAll("**", PLACEHOLDER)
    .replace(/\./g, "\\.")
    .replace(/\*/g, "[^/]*")
    .replaceAll(PLACEHOLDER, ".*");

  return new RegExp(`^${regex}$`).test(filepath);
}

async function scanArtFiles(
  root: string,
  include: string[],
  exclude: string[],
  scanInlineArt = false,
): Promise<string[]> {
  const files: string[] = [];

  async function scan(dir: string): Promise<void> {
    const entries = await fs.promises.readdir(dir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      const relative = path.relative(root, fullPath);

      // Check exclude
      let excluded = false;
      for (const pattern of exclude) {
        if (matchGlob(relative, pattern) || matchGlob(entry.name, pattern)) {
          excluded = true;
          break;
        }
      }

      if (excluded) continue;

      if (entry.isDirectory()) {
        await scan(fullPath);
      } else if (entry.isFile() && entry.name.endsWith(".art.vue")) {
        // Check include
        for (const pattern of include) {
          if (matchGlob(relative, pattern)) {
            files.push(fullPath);
            break;
          }
        }
      } else if (
        scanInlineArt &&
        entry.isFile() &&
        entry.name.endsWith(".vue") &&
        !entry.name.endsWith(".art.vue")
      ) {
        // Inline art: check if .vue file contains <art block
        const content = await fs.promises.readFile(fullPath, "utf-8");
        if (content.includes("<art")) {
          files.push(fullPath);
        }
      }
    }
  }

  await scan(root);
  return files;
}

function generateGalleryHtml(
  basePath: string,
  themeConfig?: { default: string; custom?: Record<string, unknown> },
): string {
  const themeScript = themeConfig
    ? `window.__MUSEA_THEME_CONFIG__=${JSON.stringify(themeConfig)};`
    : "";
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Musea - Component Gallery</title>
  <script>window.__MUSEA_BASE_PATH__='${basePath}';${themeScript}${"<"}/script>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
  <style>
    :root {
      --musea-bg-primary: #0d0d0d;
      --musea-bg-secondary: #1a1815;
      --musea-bg-tertiary: #252220;
      --musea-bg-elevated: #2d2a27;
      --musea-accent: #a34828;
      --musea-accent-hover: #c45a32;
      --musea-accent-subtle: rgba(163, 72, 40, 0.15);
      --musea-text: #e6e9f0;
      --musea-text-secondary: #c4c9d4;
      --musea-text-muted: #7b8494;
      --musea-border: #3a3530;
      --musea-border-subtle: #2a2725;
      --musea-success: #4ade80;
      --musea-shadow: 0 4px 24px rgba(0, 0, 0, 0.4);
      --musea-radius-sm: 6px;
      --musea-radius-md: 8px;
      --musea-radius-lg: 12px;
      --musea-transition: 0.15s ease;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; }

    body {
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
      background: var(--musea-bg-primary);
      color: var(--musea-text);
      min-height: 100vh;
      line-height: 1.5;
      -webkit-font-smoothing: antialiased;
    }

    /* Header */
    .header {
      background: var(--musea-bg-secondary);
      border-bottom: 1px solid var(--musea-border);
      padding: 0 1.5rem;
      height: 56px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      position: sticky;
      top: 0;
      z-index: 100;
    }

    .header-left {
      display: flex;
      align-items: center;
      gap: 1.5rem;
    }

    .logo {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      font-size: 1.125rem;
      font-weight: 700;
      color: var(--musea-accent);
      text-decoration: none;
    }

    .logo-svg {
      width: 32px;
      height: 32px;
      flex-shrink: 0;
    }

    .logo-icon svg {
      width: 16px;
      height: 16px;
      color: white;
    }

    .header-subtitle {
      color: var(--musea-text-muted);
      font-size: 0.8125rem;
      font-weight: 500;
      padding-left: 1.5rem;
      border-left: 1px solid var(--musea-border);
    }

    .search-container {
      position: relative;
      width: 280px;
    }

    .search-input {
      width: 100%;
      background: var(--musea-bg-tertiary);
      border: 1px solid var(--musea-border);
      border-radius: var(--musea-radius-md);
      padding: 0.5rem 0.75rem 0.5rem 2.25rem;
      color: var(--musea-text);
      font-size: 0.8125rem;
      outline: none;
      transition: border-color var(--musea-transition), background var(--musea-transition);
    }

    .search-input::placeholder {
      color: var(--musea-text-muted);
    }

    .search-input:focus {
      border-color: var(--musea-accent);
      background: var(--musea-bg-elevated);
    }

    .search-icon {
      position: absolute;
      left: 0.75rem;
      top: 50%;
      transform: translateY(-50%);
      color: var(--musea-text-muted);
      pointer-events: none;
    }

    /* Layout */
    .main {
      display: grid;
      grid-template-columns: 260px 1fr;
      min-height: calc(100vh - 56px);
    }

    /* Sidebar */
    .sidebar {
      background: var(--musea-bg-secondary);
      border-right: 1px solid var(--musea-border);
      overflow-y: auto;
      overflow-x: hidden;
    }

    .sidebar::-webkit-scrollbar {
      width: 6px;
    }

    .sidebar::-webkit-scrollbar-track {
      background: transparent;
    }

    .sidebar::-webkit-scrollbar-thumb {
      background: var(--musea-border);
      border-radius: 3px;
    }

    .sidebar-section {
      padding: 0.75rem;
    }

    .category-header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.625rem 0.75rem;
      font-size: 0.6875rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      color: var(--musea-text-muted);
      cursor: pointer;
      user-select: none;
      border-radius: var(--musea-radius-sm);
      transition: background var(--musea-transition);
    }

    .category-header:hover {
      background: var(--musea-bg-tertiary);
    }

    .category-icon {
      width: 16px;
      height: 16px;
      transition: transform var(--musea-transition);
    }

    .category-header.collapsed .category-icon {
      transform: rotate(-90deg);
    }

    .category-count {
      margin-left: auto;
      background: var(--musea-bg-tertiary);
      padding: 0.125rem 0.375rem;
      border-radius: 4px;
      font-size: 0.625rem;
    }

    .art-list {
      list-style: none;
      margin-top: 0.25rem;
    }

    .art-item {
      display: flex;
      align-items: center;
      gap: 0.625rem;
      padding: 0.5rem 0.75rem 0.5rem 1.75rem;
      border-radius: var(--musea-radius-sm);
      cursor: pointer;
      font-size: 0.8125rem;
      color: var(--musea-text-secondary);
      transition: all var(--musea-transition);
      position: relative;
    }

    .art-item::before {
      content: '';
      position: absolute;
      left: 0.75rem;
      top: 50%;
      transform: translateY(-50%);
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--musea-border);
      transition: background var(--musea-transition);
    }

    .art-item:hover {
      background: var(--musea-bg-tertiary);
      color: var(--musea-text);
    }

    .art-item:hover::before {
      background: var(--musea-text-muted);
    }

    .art-item.active {
      background: var(--musea-accent-subtle);
      color: var(--musea-accent-hover);
    }

    .art-item.active::before {
      background: var(--musea-accent);
    }

    .art-variant-count {
      margin-left: auto;
      font-size: 0.6875rem;
      color: var(--musea-text-muted);
      opacity: 0;
      transition: opacity var(--musea-transition);
    }

    .art-item:hover .art-variant-count {
      opacity: 1;
    }

    /* Content */
    .content {
      background: var(--musea-bg-primary);
      overflow-y: auto;
    }

    .content-inner {
      max-width: 1400px;
      margin: 0 auto;
      padding: 2rem;
    }

    .content-header {
      margin-bottom: 2rem;
    }

    .content-title {
      font-size: 1.5rem;
      font-weight: 700;
      margin-bottom: 0.5rem;
    }

    .content-description {
      color: var(--musea-text-muted);
      font-size: 0.9375rem;
      max-width: 600px;
    }

    .content-meta {
      display: flex;
      align-items: center;
      gap: 1rem;
      margin-top: 1rem;
    }

    .meta-tag {
      display: inline-flex;
      align-items: center;
      gap: 0.375rem;
      padding: 0.25rem 0.625rem;
      background: var(--musea-bg-secondary);
      border: 1px solid var(--musea-border);
      border-radius: var(--musea-radius-sm);
      font-size: 0.75rem;
      color: var(--musea-text-muted);
    }

    .meta-tag svg {
      width: 12px;
      height: 12px;
    }

    /* Gallery Grid */
    .gallery {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
      gap: 1.25rem;
    }

    /* Variant Card */
    .variant-card {
      background: var(--musea-bg-secondary);
      border: 1px solid var(--musea-border);
      border-radius: var(--musea-radius-lg);
      overflow: hidden;
      transition: all var(--musea-transition);
    }

    .variant-card:hover {
      border-color: var(--musea-text-muted);
      box-shadow: var(--musea-shadow);
      transform: translateY(-2px);
    }

    .variant-preview {
      aspect-ratio: 16 / 10;
      background: var(--musea-bg-tertiary);
      display: flex;
      align-items: center;
      justify-content: center;
      position: relative;
      overflow: hidden;
    }

    .variant-preview iframe {
      width: 100%;
      height: 100%;
      border: none;
      background: white;
    }

    .variant-preview-placeholder {
      color: var(--musea-text-muted);
      font-size: 0.8125rem;
      text-align: center;
      padding: 1rem;
    }

    .variant-preview-code {
      font-family: 'SF Mono', 'Fira Code', monospace;
      font-size: 0.75rem;
      color: var(--musea-text-muted);
      background: var(--musea-bg-primary);
      padding: 1rem;
      overflow: auto;
      max-height: 100%;
      width: 100%;
    }

    .variant-info {
      padding: 1rem;
      border-top: 1px solid var(--musea-border);
      display: flex;
      align-items: center;
      justify-content: space-between;
    }

    .variant-name {
      font-weight: 600;
      font-size: 0.875rem;
    }

    .variant-badge {
      font-size: 0.625rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      padding: 0.1875rem 0.5rem;
      border-radius: 4px;
      background: var(--musea-accent-subtle);
      color: var(--musea-accent);
    }

    .variant-actions {
      display: flex;
      gap: 0.5rem;
    }

    .variant-action-btn {
      width: 28px;
      height: 28px;
      border: none;
      background: var(--musea-bg-tertiary);
      border-radius: var(--musea-radius-sm);
      color: var(--musea-text-muted);
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all var(--musea-transition);
    }

    .variant-action-btn:hover {
      background: var(--musea-bg-elevated);
      color: var(--musea-text);
    }

    .variant-action-btn svg {
      width: 14px;
      height: 14px;
    }

    /* Empty State */
    .empty-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      min-height: 400px;
      text-align: center;
      padding: 2rem;
    }

    .empty-state-icon {
      width: 80px;
      height: 80px;
      background: var(--musea-bg-secondary);
      border-radius: var(--musea-radius-lg);
      display: flex;
      align-items: center;
      justify-content: center;
      margin-bottom: 1.5rem;
    }

    .empty-state-icon svg {
      width: 40px;
      height: 40px;
      color: var(--musea-text-muted);
    }

    .empty-state-title {
      font-size: 1.125rem;
      font-weight: 600;
      margin-bottom: 0.5rem;
    }

    .empty-state-text {
      color: var(--musea-text-muted);
      font-size: 0.875rem;
      max-width: 300px;
    }

    /* Loading */
    .loading {
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 200px;
      color: var(--musea-text-muted);
      gap: 0.75rem;
    }

    .loading-spinner {
      width: 20px;
      height: 20px;
      border: 2px solid var(--musea-border);
      border-top-color: var(--musea-accent);
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    /* Responsive */
    @media (max-width: 768px) {
      .main {
        grid-template-columns: 1fr;
      }
      .sidebar {
        display: none;
      }
      .header-subtitle {
        display: none;
      }
    }
  </style>
</head>
<body>
  <header class="header">
    <div class="header-left">
      <a href="${basePath}" class="logo">
        <svg class="logo-svg" width="32" height="32" viewBox="0 0 200 200" fill="none">
          <defs>
            <linearGradient id="metal-grad" x1="0%" y1="0%" x2="100%" y2="20%">
              <stop offset="0%" stop-color="#f0f2f5"/>
              <stop offset="50%" stop-color="#9ca3b0"/>
              <stop offset="100%" stop-color="#e07048"/>
            </linearGradient>
            <linearGradient id="metal-grad-dark" x1="0%" y1="0%" x2="100%" y2="30%">
              <stop offset="0%" stop-color="#d0d4dc"/>
              <stop offset="60%" stop-color="#6b7280"/>
              <stop offset="100%" stop-color="#c45530"/>
            </linearGradient>
          </defs>
          <g transform="translate(40, 40)">
            <g transform="skewX(-12)">
              <path d="M 100 0 L 60 120 L 105 30 L 100 0 Z" fill="url(#metal-grad-dark)" stroke="#4b5563" stroke-width="0.5"/>
              <path d="M 30 0 L 60 120 L 80 20 L 30 0 Z" fill="url(#metal-grad)" stroke-width="0.5" stroke-opacity="0.4"/>
            </g>
          </g>
          <g transform="translate(110, 120)">
            <line x1="5" y1="10" x2="5" y2="50" stroke="#e07048" stroke-width="3" stroke-linecap="round"/>
            <line x1="60" y1="10" x2="60" y2="50" stroke="#e07048" stroke-width="3" stroke-linecap="round"/>
            <path d="M 0 10 L 32.5 0 L 65 10" fill="none" stroke="#e07048" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>
            <rect x="15" y="18" width="14" height="12" rx="1" fill="none" stroke="#e07048" stroke-width="1.5" opacity="0.7"/>
            <rect x="36" y="18" width="14" height="12" rx="1" fill="none" stroke="#e07048" stroke-width="1.5" opacity="0.7"/>
            <rect x="23" y="35" width="18" height="12" rx="1" fill="none" stroke="#e07048" stroke-width="1.5" opacity="0.6"/>
          </g>
        </svg>
        Musea
      </a>
      <span class="header-subtitle">Component Gallery</span>
    </div>
    <div class="search-container">
      <svg class="search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
      </svg>
      <input type="text" class="search-input" placeholder="Search components..." id="search">
    </div>
  </header>

  <main class="main">
    <aside class="sidebar" id="sidebar">
      <div class="loading">
        <div class="loading-spinner"></div>
        Loading...
      </div>
    </aside>
    <section class="content" id="content">
      <div class="empty-state">
        <div class="empty-state-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M4 5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5Z"/>
            <path d="M4 13a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-6Z"/>
            <path d="M16 13a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1h-2a1 1 0 0 1-1-1v-6Z"/>
          </svg>
        </div>
        <div class="empty-state-title">Select a component</div>
        <div class="empty-state-text">Choose a component from the sidebar to view its variants and documentation</div>
      </div>
    </section>
  </main>

  <script type="module">
    const basePath = '${basePath}';
    let arts = [];
    let selectedArt = null;
    let searchQuery = '';

    async function loadArts() {
      try {
        const res = await fetch(basePath + '/api/arts');
        arts = await res.json();
        renderSidebar();
      } catch (e) {
        console.error('Failed to load arts:', e);
        document.getElementById('sidebar').innerHTML = '<div class="loading">Failed to load</div>';
      }
    }

    function renderSidebar() {
      const sidebar = document.getElementById('sidebar');
      const categories = {};

      const filtered = searchQuery
        ? arts.filter(a => a.metadata.title.toLowerCase().includes(searchQuery.toLowerCase()))
        : arts;

      for (const art of filtered) {
        const cat = art.metadata.category || 'Components';
        if (!categories[cat]) categories[cat] = [];
        categories[cat].push(art);
      }

      if (Object.keys(categories).length === 0) {
        sidebar.innerHTML = '<div class="sidebar-section"><div class="loading">No components found</div></div>';
        return;
      }

      let html = '';
      for (const [category, items] of Object.entries(categories)) {
        html += '<div class="sidebar-section">';
        html += '<div class="category-header" data-category="' + category + '">';
        html += '<svg class="category-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m9 18 6-6-6-6"/></svg>';
        html += '<span>' + category + '</span>';
        html += '<span class="category-count">' + items.length + '</span>';
        html += '</div>';
        html += '<ul class="art-list" data-category="' + category + '">';
        for (const art of items) {
          const active = selectedArt?.path === art.path ? 'active' : '';
          const variantCount = art.variants?.length || 0;
          html += '<li class="art-item ' + active + '" data-path="' + art.path + '">';
          html += '<span>' + escapeHtml(art.metadata.title) + '</span>';
          html += '<span class="art-variant-count">' + variantCount + ' variant' + (variantCount !== 1 ? 's' : '') + '</span>';
          html += '</li>';
        }
        html += '</ul>';
        html += '</div>';
      }

      sidebar.innerHTML = html;

      sidebar.querySelectorAll('.art-item').forEach(item => {
        item.addEventListener('click', () => {
          const artPath = item.dataset.path;
          selectedArt = arts.find(a => a.path === artPath);
          renderSidebar();
          renderContent();
        });
      });

      sidebar.querySelectorAll('.category-header').forEach(header => {
        header.addEventListener('click', () => {
          header.classList.toggle('collapsed');
          const list = sidebar.querySelector('.art-list[data-category="' + header.dataset.category + '"]');
          if (list) list.style.display = header.classList.contains('collapsed') ? 'none' : 'block';
        });
      });
    }

    function renderContent() {
      const content = document.getElementById('content');
      if (!selectedArt) {
        content.innerHTML = \`
          <div class="empty-state">
            <div class="empty-state-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <path d="M4 5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5Z"/>
                <path d="M4 13a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-6Z"/>
                <path d="M16 13a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1h-2a1 1 0 0 1-1-1v-6Z"/>
              </svg>
            </div>
            <div class="empty-state-title">Select a component</div>
            <div class="empty-state-text">Choose a component from the sidebar to view its variants</div>
          </div>
        \`;
        return;
      }

      const meta = selectedArt.metadata;
      const tags = meta.tags || [];
      const variantCount = selectedArt.variants?.length || 0;

      let html = '<div class="content-inner">';
      html += '<div class="content-header">';
      html += '<h1 class="content-title">' + escapeHtml(meta.title) + '</h1>';
      if (meta.description) {
        html += '<p class="content-description">' + escapeHtml(meta.description) + '</p>';
      }
      html += '<div class="content-meta">';
      html += '<span class="meta-tag"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/></svg>' + variantCount + ' variant' + (variantCount !== 1 ? 's' : '') + '</span>';
      if (meta.category) {
        html += '<span class="meta-tag"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>' + escapeHtml(meta.category) + '</span>';
      }
      for (const tag of tags) {
        html += '<span class="meta-tag">#' + escapeHtml(tag) + '</span>';
      }
      html += '</div>';
      html += '</div>';

      html += '<div class="gallery">';
      for (const variant of selectedArt.variants) {
        const previewUrl = basePath + '/preview?art=' + encodeURIComponent(selectedArt.path) + '&variant=' + encodeURIComponent(variant.name);

        html += '<div class="variant-card">';
        html += '<div class="variant-preview">';
        html += '<iframe src="' + previewUrl + '" loading="lazy" title="' + escapeHtml(variant.name) + '"></iframe>';
        html += '</div>';
        html += '<div class="variant-info">';
        html += '<div>';
        html += '<span class="variant-name">' + escapeHtml(variant.name) + '</span>';
        if (variant.isDefault) html += ' <span class="variant-badge">Default</span>';
        html += '</div>';
        html += '<div class="variant-actions">';
        html += '<button class="variant-action-btn" title="Open in new tab" onclick="window.open(\\'' + previewUrl + '\\', \\'_blank\\')"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg></button>';
        html += '</div>';
        html += '</div>';
        html += '</div>';
      }
      html += '</div>';
      html += '</div>';

      content.innerHTML = html;
    }

    function escapeHtml(str) {
      if (!str) return '';
      return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    }

    // Search
    document.getElementById('search').addEventListener('input', (e) => {
      searchQuery = e.target.value;
      renderSidebar();
    });

    // Keyboard shortcut for search
    document.addEventListener('keydown', (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        document.getElementById('search').focus();
      }
    });

    loadArts();
  </script>
</body>
</html>`;
}

function generateGalleryModule(basePath: string): string {
  return `
export const basePath = '${basePath}';
export async function loadArts() {
  const res = await fetch(basePath + '/api/arts');
  return res.json();
}
`;
}

// Addon initialization code injected into preview iframe modules.
// Shared between generatePreviewModule and generatePreviewModuleWithProps.
const MUSEA_ADDONS_INIT_CODE = `
function __museaInitAddons(container, variantName) {
  // === DOM event capture ===
  // Note: mousemove, mouseenter, mouseleave, pointermove are excluded as they are too noisy
  const CAPTURE_EVENTS = ['click','dblclick','input','change','submit','focus','blur','keydown','keyup','mousedown','mouseup','wheel','contextmenu','pointerdown','pointerup'];
  for (const evt of CAPTURE_EVENTS) {
    container.addEventListener(evt, (e) => {
      // Extract raw event properties
      const rawEvent = {
        type: e.type,
        bubbles: e.bubbles,
        cancelable: e.cancelable,
        composed: e.composed,
        defaultPrevented: e.defaultPrevented,
        eventPhase: e.eventPhase,
        isTrusted: e.isTrusted,
        timeStamp: e.timeStamp,
      };
      // Mouse/Pointer event properties
      if ('clientX' in e) {
        rawEvent.clientX = e.clientX;
        rawEvent.clientY = e.clientY;
        rawEvent.screenX = e.screenX;
        rawEvent.screenY = e.screenY;
        rawEvent.pageX = e.pageX;
        rawEvent.pageY = e.pageY;
        rawEvent.offsetX = e.offsetX;
        rawEvent.offsetY = e.offsetY;
        rawEvent.button = e.button;
        rawEvent.buttons = e.buttons;
        rawEvent.altKey = e.altKey;
        rawEvent.ctrlKey = e.ctrlKey;
        rawEvent.metaKey = e.metaKey;
        rawEvent.shiftKey = e.shiftKey;
      }
      // Keyboard event properties
      if ('key' in e) {
        rawEvent.key = e.key;
        rawEvent.code = e.code;
        rawEvent.repeat = e.repeat;
        rawEvent.altKey = e.altKey;
        rawEvent.ctrlKey = e.ctrlKey;
        rawEvent.metaKey = e.metaKey;
        rawEvent.shiftKey = e.shiftKey;
      }
      // Input event properties
      if ('inputType' in e) {
        rawEvent.inputType = e.inputType;
        rawEvent.data = e.data;
      }
      // Wheel event properties
      if ('deltaX' in e) {
        rawEvent.deltaX = e.deltaX;
        rawEvent.deltaY = e.deltaY;
        rawEvent.deltaZ = e.deltaZ;
        rawEvent.deltaMode = e.deltaMode;
      }
      const payload = {
        name: evt,
        target: e.target?.tagName,
        timestamp: Date.now(),
        source: 'dom',
        rawEvent,
        variantName
      };
      if (e.target && 'value' in e.target) {
        payload.value = e.target.value;
      }
      window.parent.postMessage({ type: 'musea:event', payload }, '*');
    }, true);
  }

  // === Message handler for parent commands ===
  let measureActive = false;
  let measureOverlay = null;
  let measureLabel = null;

  function toggleStyleById(id, enabled, css) {
    let el = document.getElementById(id);
    if (enabled) {
      if (!el) {
        el = document.createElement('style');
        el.id = id;
        el.textContent = css;
        document.head.appendChild(el);
      }
    } else {
      if (el) el.remove();
    }
  }

  function createMeasureOverlay() {
    if (measureOverlay) return;
    measureOverlay = document.createElement('div');
    measureOverlay.id = 'musea-measure-overlay';
    measureOverlay.style.cssText = 'position:fixed;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:99999;';
    document.body.appendChild(measureOverlay);

    measureLabel = document.createElement('div');
    measureLabel.className = 'musea-measure-label';
    measureLabel.style.cssText = 'position:fixed;background:#333;color:#fff;font-size:11px;padding:2px 6px;border-radius:3px;pointer-events:none;z-index:100000;display:none;';
    document.body.appendChild(measureLabel);
  }

  function removeMeasureOverlay() {
    if (measureOverlay) { measureOverlay.remove(); measureOverlay = null; }
    if (measureLabel) { measureLabel.remove(); measureLabel = null; }
  }

  function onMeasureMouseMove(e) {
    if (!measureActive || !measureOverlay) return;
    const el = document.elementFromPoint(e.clientX, e.clientY);
    if (!el || el === measureOverlay || el === measureLabel) return;

    const rect = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    const mt = parseFloat(cs.marginTop) || 0;
    const mr = parseFloat(cs.marginRight) || 0;
    const mb = parseFloat(cs.marginBottom) || 0;
    const ml = parseFloat(cs.marginLeft) || 0;
    const bt = parseFloat(cs.borderTopWidth) || 0;
    const br = parseFloat(cs.borderRightWidth) || 0;
    const bb = parseFloat(cs.borderBottomWidth) || 0;
    const blw = parseFloat(cs.borderLeftWidth) || 0;
    const pt = parseFloat(cs.paddingTop) || 0;
    const pr = parseFloat(cs.paddingRight) || 0;
    const pb = parseFloat(cs.paddingBottom) || 0;
    const pl = parseFloat(cs.paddingLeft) || 0;

    const cw = rect.width - blw - br - pl - pr;
    const ch = rect.height - bt - bb - pt - pb;

    measureOverlay.innerHTML = ''
      // Margin
      + '<div style="position:fixed;background:rgba(255,165,0,0.3);'
      + 'left:' + (rect.left - ml) + 'px;top:' + (rect.top - mt) + 'px;'
      + 'width:' + (rect.width + ml + mr) + 'px;height:' + mt + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(255,165,0,0.3);'
      + 'left:' + (rect.left - ml) + 'px;top:' + (rect.bottom) + 'px;'
      + 'width:' + (rect.width + ml + mr) + 'px;height:' + mb + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(255,165,0,0.3);'
      + 'left:' + (rect.left - ml) + 'px;top:' + rect.top + 'px;'
      + 'width:' + ml + 'px;height:' + rect.height + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(255,165,0,0.3);'
      + 'left:' + rect.right + 'px;top:' + rect.top + 'px;'
      + 'width:' + mr + 'px;height:' + rect.height + 'px;"></div>'
      // Border
      + '<div style="position:fixed;background:rgba(255,255,0,0.3);'
      + 'left:' + rect.left + 'px;top:' + rect.top + 'px;'
      + 'width:' + rect.width + 'px;height:' + bt + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(255,255,0,0.3);'
      + 'left:' + rect.left + 'px;top:' + (rect.bottom - bb) + 'px;'
      + 'width:' + rect.width + 'px;height:' + bb + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(255,255,0,0.3);'
      + 'left:' + rect.left + 'px;top:' + (rect.top + bt) + 'px;'
      + 'width:' + blw + 'px;height:' + (rect.height - bt - bb) + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(255,255,0,0.3);'
      + 'left:' + (rect.right - br) + 'px;top:' + (rect.top + bt) + 'px;'
      + 'width:' + br + 'px;height:' + (rect.height - bt - bb) + 'px;"></div>'
      // Padding
      + '<div style="position:fixed;background:rgba(144,238,144,0.3);'
      + 'left:' + (rect.left + blw) + 'px;top:' + (rect.top + bt) + 'px;'
      + 'width:' + (rect.width - blw - br) + 'px;height:' + pt + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(144,238,144,0.3);'
      + 'left:' + (rect.left + blw) + 'px;top:' + (rect.bottom - bb - pb) + 'px;'
      + 'width:' + (rect.width - blw - br) + 'px;height:' + pb + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(144,238,144,0.3);'
      + 'left:' + (rect.left + blw) + 'px;top:' + (rect.top + bt + pt) + 'px;'
      + 'width:' + pl + 'px;height:' + (rect.height - bt - bb - pt - pb) + 'px;"></div>'
      + '<div style="position:fixed;background:rgba(144,238,144,0.3);'
      + 'left:' + (rect.right - br - pr) + 'px;top:' + (rect.top + bt + pt) + 'px;'
      + 'width:' + pr + 'px;height:' + (rect.height - bt - bb - pt - pb) + 'px;"></div>'
      // Content
      + '<div style="position:fixed;background:rgba(100,149,237,0.3);'
      + 'left:' + (rect.left + blw + pl) + 'px;top:' + (rect.top + bt + pt) + 'px;'
      + 'width:' + cw + 'px;height:' + ch + 'px;"></div>';

    // Label
    measureLabel.textContent = Math.round(rect.width) + ' x ' + Math.round(rect.height);
    measureLabel.style.display = 'block';
    measureLabel.style.left = (rect.right + 8) + 'px';
    measureLabel.style.top = rect.top + 'px';
  }

  window.addEventListener('message', (e) => {
    if (!e.data?.type?.startsWith('musea:')) return;
    const { type, payload } = e.data;
    switch (type) {
      case 'musea:set-background': {
        if (payload.pattern === 'checkerboard') {
          document.body.style.background = '';
          document.body.classList.add('musea-bg-checkerboard');
        } else {
          document.body.classList.remove('musea-bg-checkerboard');
          document.body.style.background = payload.color || '';
        }
        break;
      }
      case 'musea:toggle-outline': {
        toggleStyleById('musea-outline', payload.enabled,
          '* { outline: 1px solid rgba(255, 0, 0, 0.3) !important; }');
        break;
      }
      case 'musea:toggle-measure': {
        measureActive = payload.enabled;
        if (measureActive) {
          createMeasureOverlay();
          document.addEventListener('mousemove', onMeasureMouseMove);
        } else {
          document.removeEventListener('mousemove', onMeasureMouseMove);
          removeMeasureOverlay();
        }
        break;
      }
      case 'musea:set-props': {
        // Store props for remount - handled by preview module
        if (window.__museaSetProps) {
          window.__museaSetProps(payload.props || {});
        }
        break;
      }
      case 'musea:set-slots': {
        // Store slots for remount - handled by preview module
        if (window.__museaSetSlots) {
          window.__museaSetSlots(payload.slots || {});
        }
        break;
      }
      case 'musea:run-a11y': {
        // Run axe-core a11y test
        (async () => {
          try {
            // Dynamically load axe-core from CDN if not already loaded
            if (!window.axe) {
              const script = document.createElement('script');
              script.src = 'https://cdnjs.cloudflare.com/ajax/libs/axe-core/4.10.2/axe.min.js';
              script.crossOrigin = 'anonymous';
              await new Promise((resolve, reject) => {
                script.onload = resolve;
                script.onerror = reject;
                document.head.appendChild(script);
              });
            }
            // Run axe-core on the .musea-variant container only (not the full document)
            const context = document.querySelector('.musea-variant') || document;
            const results = await window.axe.run(context, {
              // Run all rules without restrictions for comprehensive testing
              resultTypes: ['violations', 'incomplete', 'passes']
            });
            window.parent.postMessage({
              type: 'musea:a11y-result',
              payload: {
                violations: results.violations.map(v => ({
                  id: v.id,
                  impact: v.impact,
                  description: v.description,
                  helpUrl: v.helpUrl,
                  nodes: v.nodes.map(n => ({
                    html: n.html,
                    target: n.target,
                    failureSummary: n.failureSummary
                  }))
                })),
                passes: results.passes.length,
                incomplete: results.incomplete.length
              }
            }, '*');
          } catch (err) {
            window.parent.postMessage({
              type: 'musea:a11y-result',
              payload: {
                error: err instanceof Error ? err.message : String(err),
                violations: [],
                passes: 0,
                incomplete: 0
              }
            }, '*');
          }
        })();
        break;
      }
    }
  });

  // Notify parent that iframe is ready
  window.parent.postMessage({ type: 'musea:ready', payload: {} }, '*');
}
`;

function generatePreviewModule(
  art: ArtFileInfo,
  variantComponentName: string,
  variantName: string,
  cssImports: string[] = [],
  previewSetup: string | null = null,
): string {
  const artModuleId = `virtual:musea-art:${art.path}`;
  const escapedVariantName = escapeTemplate(variantName);
  const cssImportStatements = cssImports.map((cssPath) => `import '${cssPath}';`).join("\n");
  const setupImport = previewSetup ? `import __museaPreviewSetup from '${previewSetup}';` : "";
  const setupCall = previewSetup ? "await __museaPreviewSetup(app);" : "";

  return `
${cssImportStatements}
${setupImport}
import { createApp, reactive, h } from 'vue';
import * as artModule from '${artModuleId}';

const container = document.getElementById('app');

${MUSEA_ADDONS_INIT_CODE}

let currentApp = null;
const propsOverride = reactive({});
const slotsOverride = reactive({ default: '' });

window.__museaSetProps = (props) => {
  // Clear old keys
  for (const key of Object.keys(propsOverride)) {
    delete propsOverride[key];
  }
  Object.assign(propsOverride, props);
};

window.__museaSetSlots = (slots) => {
  Object.assign(slotsOverride, slots);
};

async function mount() {
  try {
    // Get the specific variant component
    const VariantComponent = artModule['${variantComponentName}'];
    const RawComponent = artModule.__component__;

    if (!VariantComponent) {
      throw new Error('Variant component "${variantComponentName}" not found in art module');
    }

    // Create and mount the app
    const app = createApp(VariantComponent);
    ${setupCall}
    container.innerHTML = '';
    container.className = 'musea-variant';
    app.mount(container);
    currentApp = app;

    console.log('[musea-preview] Mounted variant: ${escapedVariantName}');
    __museaInitAddons(container, '${escapedVariantName}');

    // Override set-props to remount with raw component + props
    if (RawComponent) {
      window.__museaSetProps = (props) => {
        for (const key of Object.keys(propsOverride)) {
          delete propsOverride[key];
        }
        Object.assign(propsOverride, props);
        remountWithProps(RawComponent);
      };
      window.__museaSetSlots = (slots) => {
        Object.assign(slotsOverride, slots);
        remountWithProps(RawComponent);
      };
    }
  } catch (error) {
    console.error('[musea-preview] Failed to mount:', error);
    container.innerHTML = \`
      <div class="musea-error">
        <div class="musea-error-title">Failed to render component</div>
        <div>\${error.message}</div>
        <pre>\${error.stack || ''}</pre>
      </div>
    \`;
  }
}

async function remountWithProps(Component) {
  if (currentApp) {
    currentApp.unmount();
  }
  const app = createApp({
    setup() {
      return () => {
        const slotFns = {};
        if (slotsOverride.default) {
          slotFns.default = () => h('span', { innerHTML: slotsOverride.default });
        }
        return h(Component, { ...propsOverride }, slotFns);
      };
    }
  });
  ${setupCall}
  container.innerHTML = '';
  app.mount(container);
  currentApp = app;
}

mount();
`;
}

function generateManifestModule(artFiles: Map<string, ArtFileInfo>): string {
  const arts = Array.from(artFiles.values());
  return `export const arts = ${JSON.stringify(arts, null, 2)};`;
}

/**
 * Extract the content of the first <script setup> block from a Vue SFC source.
 */
function extractScriptSetupContent(source: string): string | undefined {
  const match = source.match(/<script\s+[^>]*setup[^>]*>([\s\S]*?)<\/script>/);
  return match?.[1]?.trim();
}

/**
 * Parse script setup content into imports and setup body.
 * Returns the import lines, setup body lines, and all identifiers to expose.
 */
function parseScriptSetupForArt(content: string): {
  imports: string[];
  setupBody: string[];
  returnNames: string[];
} {
  const lines = content.split("\n");
  const imports: string[] = [];
  const setupBody: string[] = [];
  const returnNames: Set<string> = new Set();

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//")) continue;

    if (trimmed.startsWith("import ")) {
      imports.push(line);
      // Extract imported names for the return statement
      const defaultMatch = trimmed.match(/^import\s+(\w+)/);
      if (defaultMatch && defaultMatch[1] !== "type") {
        returnNames.add(defaultMatch[1]);
      }
      const namedMatch = trimmed.match(/\{([^}]+)\}/);
      if (namedMatch) {
        for (const part of namedMatch[1].split(",")) {
          const name = part
            .trim()
            .split(/\s+as\s+/)
            .pop()
            ?.trim();
          if (name && !name.startsWith("type ")) {
            returnNames.add(name);
          }
        }
      }
    } else {
      setupBody.push(line);
      // Extract declared variable names
      const constMatch = trimmed.match(/^(?:const|let|var)\s+(\w+)/);
      if (constMatch) {
        returnNames.add(constMatch[1]);
      }
      // Handle destructuring: const { a, b } = ...
      const destructMatch = trimmed.match(/^(?:const|let|var)\s+\{([^}]+)\}/);
      if (destructMatch) {
        for (const part of destructMatch[1].split(",")) {
          const name = part
            .trim()
            .split(/\s*:\s*/)
            .shift()
            ?.trim();
          if (name) returnNames.add(name);
        }
      }
      // Handle array destructuring: const [a, b] = ...
      const arrayMatch = trimmed.match(/^(?:const|let|var)\s+\[([^\]]+)\]/);
      if (arrayMatch) {
        for (const part of arrayMatch[1].split(",")) {
          const name = part.trim();
          if (name && name !== "...") returnNames.add(name);
        }
      }
    }
  }

  // Remove 'type' keyword imports and common Vue utilities that shouldn't be returned
  returnNames.delete("type");

  return {
    imports,
    setupBody,
    returnNames: [...returnNames],
  };
}

function generateArtModule(art: ArtFileInfo, filePath: string): string {
  let componentImportPath: string | undefined;
  let componentName: string | undefined;

  if (art.isInline && art.componentPath) {
    // Inline art: import the host .vue file itself as the component
    componentImportPath = art.componentPath;
    componentName = path.basename(art.componentPath, ".vue");
  } else if (art.metadata.component) {
    // Traditional .art.vue: resolve component from the component attribute
    const comp = art.metadata.component;
    componentImportPath = path.isAbsolute(comp) ? comp : path.resolve(path.dirname(filePath), comp);
    componentName = path.basename(comp, ".vue");
  }

  // Parse script setup if present
  const scriptSetup = art.scriptSetupContent
    ? parseScriptSetupForArt(art.scriptSetupContent)
    : null;

  let code = `
// Auto-generated module for: ${path.basename(filePath)}
import { defineComponent, h } from 'vue';
`;

  // Add script setup imports at module level
  // Resolve relative paths to absolute since this code runs inside a virtual module
  if (scriptSetup) {
    const artDir = path.dirname(filePath);
    for (const imp of scriptSetup.imports) {
      const resolved = imp.replace(/from\s+(['"])(\.[^'"]+)\1/, (_match, quote, relPath) => {
        const absPath = path.resolve(artDir, relPath);
        return `from ${quote}${absPath}${quote}`;
      });
      code += `${resolved}\n`;
    }
  }

  if (componentImportPath && componentName) {
    // Only add component import if not already imported by script setup
    const alreadyImported = scriptSetup?.imports.some((imp) => {
      // Check against the original relative path and the resolved absolute path
      if (
        imp.includes(`from '${componentImportPath}'`) ||
        imp.includes(`from "${componentImportPath}"`)
      )
        return true;
      // Also check by component name as default import (handles relative vs absolute path mismatch)
      return new RegExp(`^import\\s+${componentName}[\\s,]`).test(imp.trim());
    });
    if (!alreadyImported) {
      code += `import ${componentName} from '${componentImportPath}';\n`;
    }
    code += `export const __component__ = ${componentName};\n`;
  }

  code += `
export const metadata = ${JSON.stringify(art.metadata)};
export const variants = ${JSON.stringify(art.variants)};
`;

  // Generate variant components
  for (const variant of art.variants) {
    const variantComponentName = toPascalCase(variant.name);

    let template = variant.template;

    // Replace <Self> with the actual component name (for inline art)
    if (componentName) {
      template = template
        .replace(/<Self/g, `<${componentName}`)
        .replace(/<\/Self>/g, `</${componentName}>`);
    }

    // Escape the template for use in a JS string
    const escapedTemplate = template
      .replace(/\\/g, "\\\\")
      .replace(/`/g, "\\`")
      .replace(/\$/g, "\\$");

    // Wrap template with the variant container (no .musea-variant class — the
    // outer mount container already carries it; duplicating causes double padding)
    const fullTemplate = `<div data-variant="${variant.name}">${escapedTemplate}</div>`;

    // Collect component names for the `components` option.
    // Runtime-compiled templates use resolveComponent() which checks the
    // `components` option, NOT setup return values.
    const componentNames = new Set<string>();
    if (componentName) componentNames.add(componentName);
    if (scriptSetup) {
      for (const name of scriptSetup.returnNames) {
        // PascalCase names starting with uppercase are likely components
        if (/^[A-Z]/.test(name)) componentNames.add(name);
      }
    }
    const components =
      componentNames.size > 0 ? `  components: { ${[...componentNames].join(", ")} },\n` : "";

    if (scriptSetup && scriptSetup.setupBody.length > 0) {
      // Generate variant with setup function from art file's <script setup>
      code += `
export const ${variantComponentName} = defineComponent({
  name: '${variantComponentName}',
${components}  setup() {
${scriptSetup.setupBody.map((l) => `    ${l}`).join("\n")}
    return { ${scriptSetup.returnNames.join(", ")} };
  },
  template: \`${fullTemplate}\`,
});
`;
    } else if (componentName) {
      code += `
export const ${variantComponentName} = {
  name: '${variantComponentName}',
${components}  template: \`${fullTemplate}\`,
};
`;
    } else {
      code += `
export const ${variantComponentName} = {
  name: '${variantComponentName}',
  template: \`${fullTemplate}\`,
};
`;
    }
  }

  // Default export
  const defaultVariant = art.variants.find((v) => v.isDefault) || art.variants[0];
  if (defaultVariant) {
    code += `
export default ${toPascalCase(defaultVariant.name)};
`;
  }

  return code;
}

async function generateStorybookFiles(
  artFiles: Map<string, ArtFileInfo>,
  root: string,
  outDir: string,
): Promise<void> {
  const binding = loadNative();
  const outputDir = path.resolve(root, outDir);

  // Ensure output directory exists
  await fs.promises.mkdir(outputDir, { recursive: true });

  for (const [filePath, _art] of artFiles) {
    try {
      const source = await fs.promises.readFile(filePath, "utf-8");
      const csf = binding.artToCsf(source, { filename: filePath });

      const outputPath = path.join(outputDir, csf.filename);
      await fs.promises.writeFile(outputPath, csf.code, "utf-8");

      console.log(`[musea] Generated: ${path.relative(root, outputPath)}`);
    } catch (e) {
      console.error(`[musea] Failed to generate CSF for ${filePath}:`, e);
    }
  }
}

function toPascalCase(str: string): string {
  return str
    .split(/[\s\-_]+/)
    .filter(Boolean)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join("");
}

function escapeTemplate(str: string): string {
  return str.replace(/\\/g, "\\\\").replace(/'/g, "\\'").replace(/\n/g, "\\n");
}

function generatePreviewModuleWithProps(
  art: ArtFileInfo,
  variantComponentName: string,
  variantName: string,
  propsOverride: Record<string, unknown>,
  cssImports: string[] = [],
  previewSetup: string | null = null,
): string {
  const artModuleId = `virtual:musea-art:${art.path}`;
  const escapedVariantName = escapeTemplate(variantName);
  const propsJson = JSON.stringify(propsOverride);
  const cssImportStatements = cssImports.map((cssPath) => `import '${cssPath}';`).join("\n");
  const setupImport = previewSetup ? `import __museaPreviewSetup from '${previewSetup}';` : "";
  const setupCall = previewSetup ? "await __museaPreviewSetup(app);" : "";

  return `
${cssImportStatements}
${setupImport}
import { createApp, h } from 'vue';
import * as artModule from '${artModuleId}';

const container = document.getElementById('app');
const propsOverride = ${propsJson};

${MUSEA_ADDONS_INIT_CODE}

async function mount() {
  try {
    const VariantComponent = artModule['${variantComponentName}'];
    if (!VariantComponent) {
      throw new Error('Variant component "${variantComponentName}" not found');
    }

    const WrappedComponent = {
      render() {
        return h(VariantComponent, propsOverride);
      }
    };

    const app = createApp(WrappedComponent);
    ${setupCall}
    container.innerHTML = '';
    container.className = 'musea-variant';
    app.mount(container);
    console.log('[musea-preview] Mounted variant: ${escapedVariantName} with props override');
    __museaInitAddons(container, '${escapedVariantName}');
  } catch (error) {
    console.error('[musea-preview] Failed to mount:', error);
    container.innerHTML = '<div class="musea-error"><div class="musea-error-title">Failed to render</div><div>' + error.message + '</div></div>';
  }
}

mount();
`;
}

function generatePreviewHtml(
  art: ArtFileInfo,
  variant: ArtVariant,
  _basePath: string,
  viteBase?: string,
): string {
  // Use preview-module HTTP endpoint instead of virtual module import.
  // Virtual module imports in inline scripts require transformIndexHtml,
  // which creates malformed html-proxy URLs when the page URL has query params.
  const previewModuleUrl = `${_basePath}/preview-module?art=${encodeURIComponent(art.path)}&variant=${encodeURIComponent(variant.name)}`;
  const base = (viteBase || "/").replace(/\/$/, "");

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${escapeHtml(art.metadata.title)} - ${escapeHtml(variant.name)}</title>
  <script type="module" src="${base}/@vite/client"></script>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    html, body {
      width: 100%;
      height: 100%;
    }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: #ffffff;
    }
    .musea-variant {
      min-height: 100vh;
    }
    .musea-error {
      color: #dc2626;
      background: #fef2f2;
      border: 1px solid #fecaca;
      border-radius: 8px;
      padding: 1rem;
      font-size: 0.875rem;
      max-width: 400px;
    }
    .musea-error-title {
      font-weight: 600;
      margin-bottom: 0.5rem;
    }
    .musea-error pre {
      font-family: monospace;
      font-size: 0.75rem;
      white-space: pre-wrap;
      word-break: break-all;
      margin-top: 0.5rem;
      padding: 0.5rem;
      background: #fff;
      border-radius: 4px;
    }
    .musea-loading {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      color: #6b7280;
      font-size: 0.875rem;
    }
    .musea-spinner {
      width: 20px;
      height: 20px;
      border: 2px solid #e5e7eb;
      border-top-color: #3b82f6;
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
    }
    @keyframes spin { to { transform: rotate(360deg); } }

    /* Musea Addons: Checkerboard background for transparent mode */
    .musea-bg-checkerboard {
      background-image:
        linear-gradient(45deg, #ccc 25%, transparent 25%),
        linear-gradient(-45deg, #ccc 25%, transparent 25%),
        linear-gradient(45deg, transparent 75%, #ccc 75%),
        linear-gradient(-45deg, transparent 75%, #ccc 75%) !important;
      background-size: 20px 20px !important;
      background-position: 0 0, 0 10px, 10px -10px, -10px 0 !important;
    }

    /* Musea Addons: Measure label */
    .musea-measure-label {
      position: fixed;
      background: #333;
      color: #fff;
      font-size: 11px;
      padding: 2px 6px;
      border-radius: 3px;
      pointer-events: none;
      z-index: 100000;
    }
  </style>
</head>
<body>
  <div id="app" class="musea-variant" data-art="${escapeHtml(art.path)}" data-variant="${escapeHtml(variant.name)}">
    <div class="musea-loading">
      <div class="musea-spinner"></div>
      Loading component...
    </div>
  </div>
  <script type="module" src="${previewModuleUrl}"></script>
</body>
</html>`;
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#x27;");
}

export default musea;
