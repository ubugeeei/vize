/**
 * Vite plugin for Musea - Component gallery for Vue components.
 *
 * @example
 * ```ts
 * import { defineConfig } from 'vite';
 * import vue from '@vitejs/plugin-vue';
 * import { musea } from '@vizejs/vite-plugin-musea';
 *
 * export default defineConfig({
 *   plugins: [vue(), musea()],
 * });
 * ```
 */

import type { Plugin, ViteDevServer, ResolvedConfig } from "vite";
import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";

import type { MuseaOptions, ArtFileInfo, ArtMetadata, ArtVariant, CsfOutput } from "./types.js";

export type {
  MuseaOptions,
  ArtFileInfo,
  ArtMetadata,
  ArtVariant,
  CsfOutput,
  VrtOptions,
  ViewportConfig,
} from "./types.js";

export {
  MuseaVrtRunner,
  generateVrtReport,
  generateVrtJsonReport,
  type VrtResult,
  type VrtSummary,
} from "./vrt.js";

export {
  processStyleDictionary,
  parseTokens,
  generateTokensHtml,
  generateTokensMarkdown,
  type DesignToken,
  type TokenCategory,
  type StyleDictionaryConfig,
  type StyleDictionaryOutput,
} from "./style-dictionary.js";

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
      is_default: boolean;
      skip_vrt: boolean;
    }>;
    has_script_setup: boolean;
    has_script: boolean;
    style_count: number;
  };
  artToCsf: (
    source: string,
    options?: { filename?: string },
  ) => {
    code: string;
    filename: string;
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
    throw new Error(`Failed to load @vizejs/native. Make sure it's installed and built:\n${String(e)}`);
  }
}

/**
 * Create Musea Vite plugin.
 */
export function musea(options: MuseaOptions = {}): Plugin[] {
  const include = options.include ?? ["**/*.art.vue"];
  const exclude = options.exclude ?? ["node_modules/**", "dist/**"];
  const basePath = options.basePath ?? "/__musea__";
  const storybookCompat = options.storybookCompat ?? false;
  const storybookOutDir = options.storybookOutDir ?? ".storybook/stories";

  let config: ResolvedConfig;
  let server: ViteDevServer | null = null;
  const artFiles = new Map<string, ArtFileInfo>();

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
    },

    configureServer(devServer) {
      server = devServer;

      // Gallery UI route
      devServer.middlewares.use(basePath, async (req, res, next) => {
        if (req.url === "/" || req.url === "/index.html") {
          const html = generateGalleryHtml(basePath);
          res.setHeader("Content-Type", "text/html");
          res.end(html);
          return;
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
        const moduleCode = generatePreviewModule(art, variantComponentName, variant.name);

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

        const rawHtml = generatePreviewHtml(art, variant, basePath);
        // Transform HTML through Vite to properly resolve module imports
        const html = await devServer.transformIndexHtml(
          `${basePath}/preview?art=${encodeURIComponent(artPath)}&variant=${encodeURIComponent(variantName)}`,
          rawHtml,
        );
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
        // GET /api/arts - List all arts
        if (req.url === "/arts" && req.method === "GET") {
          res.setHeader("Content-Type", "application/json");
          res.end(JSON.stringify(Array.from(artFiles.values())));
          return;
        }

        // GET /api/arts/:path - Get single art
        if (req.url?.startsWith("/arts/") && req.method === "GET") {
          const artPath = decodeURIComponent(req.url.slice(6));
          const art = artFiles.get(artPath);
          if (art) {
            res.setHeader("Content-Type", "application/json");
            res.end(JSON.stringify(art));
          } else {
            res.statusCode = 404;
            res.end(JSON.stringify({ error: "Art not found" }));
          }
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
      });

      devServer.watcher.on("add", async (file) => {
        if (file.endsWith(".art.vue") && shouldProcess(file, include, exclude, config.root)) {
          await processArtFile(file);
          console.log(`[musea] Added: ${path.relative(config.root, file)}`);
        }
      });

      devServer.watcher.on("unlink", (file) => {
        if (artFiles.has(file)) {
          artFiles.delete(file);
          console.log(`[musea] Removed: ${path.relative(config.root, file)}`);
        }
      });
    },

    async buildStart() {
      // Scan for Art files
      const files = await scanArtFiles(config.root, include, exclude);

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
      if (id.startsWith("virtual:musea-art:")) {
        const artPath = id.slice("virtual:musea-art:".length);
        if (artFiles.has(artPath)) {
          return "\0musea-art:" + artPath;
        }
      }
      if (id.endsWith(".art.vue")) {
        const resolved = path.resolve(config.root, id);
        if (artFiles.has(resolved)) {
          return VIRTUAL_MUSEA_PREFIX + resolved;
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
            return generatePreviewModule(art, variantComponentName, variantName);
          }
        }
      }
      // Handle \0musea-art: prefix for preview modules
      if (id.startsWith("\0musea-art:")) {
        const artPath = id.slice("\0musea-art:".length);
        const art = artFiles.get(artPath);
        if (art) {
          return generateArtModule(art, artPath);
        }
      }
      if (id.startsWith(VIRTUAL_MUSEA_PREFIX)) {
        const realPath = id.slice(VIRTUAL_MUSEA_PREFIX.length);
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

      const info: ArtFileInfo = {
        path: filePath,
        metadata: {
          title: parsed.metadata.title,
          description: parsed.metadata.description,
          component: parsed.metadata.component,
          category: parsed.metadata.category,
          tags: parsed.metadata.tags,
          status: parsed.metadata.status as "draft" | "ready" | "deprecated",
          order: parsed.metadata.order,
        },
        variants: parsed.variants.map((v) => ({
          name: v.name,
          template: v.template,
          isDefault: v.is_default,
          skipVrt: v.skip_vrt,
        })),
        hasScriptSetup: parsed.has_script_setup,
        hasScript: parsed.has_script,
        styleCount: parsed.style_count,
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
  // Escape . first, then replace glob patterns
  const regex = pattern
    .replace(/\./g, "\\.")
    .replace(/\*\*/g, ".*")
    .replace(/\*(?!\*)/g, "[^/]*");

  return new RegExp(`^${regex}$`).test(filepath);
}

async function scanArtFiles(root: string, include: string[], exclude: string[]): Promise<string[]> {
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
      }
    }
  }

  await scan(root);
  return files;
}

function generateGalleryHtml(basePath: string): string {
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Musea - Component Gallery</title>
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

function generatePreviewModule(
  art: ArtFileInfo,
  variantComponentName: string,
  variantName: string,
): string {
  const artModuleId = `virtual:musea-art:${art.path}`;
  const escapedVariantName = escapeTemplate(variantName);

  return `
import { createApp } from 'vue';
import * as artModule from '${artModuleId}';

const container = document.getElementById('app');

async function mount() {
  try {
    // Get the specific variant component
    const VariantComponent = artModule['${variantComponentName}'];

    if (!VariantComponent) {
      throw new Error('Variant component "${variantComponentName}" not found in art module');
    }

    // Create and mount the app
    const app = createApp(VariantComponent);
    container.innerHTML = '';
    container.className = 'musea-variant';
    app.mount(container);

    console.log('[musea-preview] Mounted variant: ${escapedVariantName}');
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

mount();
`;
}

function generateManifestModule(artFiles: Map<string, ArtFileInfo>): string {
  const arts = Array.from(artFiles.values());
  return `export const arts = ${JSON.stringify(arts, null, 2)};`;
}

function generateArtModule(art: ArtFileInfo, filePath: string): string {
  const componentPath = art.metadata.component;

  // Resolve component path relative to art file location
  let resolvedComponentPath = componentPath;
  if (componentPath && !path.isAbsolute(componentPath)) {
    const artDir = path.dirname(filePath);
    resolvedComponentPath = path.resolve(artDir, componentPath);
  }

  // Extract component name from path (e.g., './Button.vue' -> 'Button')
  const componentName = componentPath ? path.basename(componentPath, ".vue") : null;

  let code = `
// Auto-generated module for: ${path.basename(filePath)}
import { defineComponent, h } from 'vue';
`;

  if (resolvedComponentPath && componentName) {
    code += `import ${componentName} from '${resolvedComponentPath}';\n`;
  }

  code += `
export const metadata = ${JSON.stringify(art.metadata)};
export const variants = ${JSON.stringify(art.variants)};
`;

  // Generate variant components
  for (const variant of art.variants) {
    const variantComponentName = toPascalCase(variant.name);
    // Escape the template for use in a JS string
    const escapedTemplate = variant.template
      .replace(/\\/g, "\\\\")
      .replace(/`/g, "\\`")
      .replace(/\$/g, "\\$");

    // Wrap template with the variant container
    const fullTemplate = `<div class="musea-variant" data-variant="${variant.name}">${escapedTemplate}</div>`;

    if (componentName) {
      code += `
export const ${variantComponentName} = {
  name: '${variantComponentName}',
  components: { ${componentName} },
  template: \`${fullTemplate}\`,
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

function generatePreviewHtml(art: ArtFileInfo, variant: ArtVariant, basePath: string): string {
  // Create a unique module URL for each variant to avoid caching issues
  const previewModuleUrl = `${basePath}/preview-module?art=${encodeURIComponent(art.path)}&variant=${encodeURIComponent(variant.name)}`;

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${escapeHtml(art.metadata.title)} - ${escapeHtml(variant.name)}</title>
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
      padding: 1.5rem;
      display: flex;
      align-items: center;
      justify-content: center;
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
