import type { Plugin, ResolvedConfig, ViteDevServer, HmrContext, TransformResult } from "vite";
import { transformWithOxc } from "vite";
import path from "node:path";
import fs from "node:fs";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import { glob } from "tinyglobby";
import type { VizeConfig, ConfigEnv, UserConfigExport, LoadConfigOptions } from "./types.js";
import type { VizeOptions, CompiledModule } from "./types.js";
import { compileFile, compileBatch } from "./compiler.js";
import { createFilter, generateOutput } from "./utils.js";
import { detectHmrUpdateType, type HmrUpdateType } from "./hmr.js";

export type { VizeOptions, CompiledModule, VizeConfig, LoadConfigOptions };

// ============================================================================
// Config utilities
// ============================================================================

const CONFIG_FILES = ["vize.config.ts", "vize.config.js", "vize.config.mjs", "vize.config.json"];

const DEFAULT_CONFIG_ENV: ConfigEnv = {
  mode: "development",
  command: "serve",
};

/**
 * Define a Vize configuration with type checking.
 * Accepts a plain object or a function that receives ConfigEnv.
 */
export function defineConfig(config: UserConfigExport): UserConfigExport {
  return config;
}

/**
 * Load Vize configuration from file
 */
export async function loadConfig(
  root: string,
  options: LoadConfigOptions = {},
): Promise<VizeConfig | null> {
  const { mode = "root", configFile, env } = options;

  if (mode === "none") return null;

  if (configFile) {
    const configPath = path.isAbsolute(configFile) ? configFile : path.resolve(root, configFile);
    return loadConfigFile(configPath, env);
  }

  if (mode === "auto") {
    let searchDir = root;
    while (true) {
      const found = findConfigInDir(searchDir);
      if (found) return loadConfigFile(found, env);
      const parentDir = path.dirname(searchDir);
      if (parentDir === searchDir) break;
      searchDir = parentDir;
    }
    return null;
  }

  // mode === "root"
  const found = findConfigInDir(root);
  return found ? loadConfigFile(found, env) : null;
}

function findConfigInDir(dir: string): string | null {
  for (const filename of CONFIG_FILES) {
    const configPath = path.join(dir, filename);
    if (fs.existsSync(configPath)) return configPath;
  }
  return null;
}

async function resolveConfigExport(
  exported: UserConfigExport,
  env?: ConfigEnv,
): Promise<VizeConfig> {
  if (typeof exported === "function") {
    return exported(env ?? DEFAULT_CONFIG_ENV);
  }
  return exported;
}

async function loadConfigFile(configPath: string, env?: ConfigEnv): Promise<VizeConfig | null> {
  if (!fs.existsSync(configPath)) return null;

  const ext = path.extname(configPath);

  if (ext === ".json") {
    const content = fs.readFileSync(configPath, "utf-8");
    return JSON.parse(content) as VizeConfig;
  }

  try {
    const module = await import(configPath);
    const exported: UserConfigExport = module.default ?? module;
    return resolveConfigExport(exported, env);
  } catch (e) {
    console.warn(`[vize] Failed to load config from ${configPath}:`, e);
    return null;
  }
}

/**
 * Shared config store for inter-plugin communication.
 * Key = project root, Value = resolved VizeConfig.
 * Used by musea() and other plugins to access the unified config.
 */
export const vizeConfigStore = new Map<string, VizeConfig>();

// Virtual module helpers.
// Module ID format: "\0/absolute/path/Component.vue.ts"
//   - \0 prefix marks it as virtual (Rollup convention), prevents other plugins from processing
//   - .ts suffix triggers TypeScript stripping in our transform hook
//   - Vite displays as "/absolute/path/Component.vue.ts" (strips \0 for logging)
// The legacy "\0vize:" prefix is only kept for CSS extraction and backward compat.
const LEGACY_VIZE_PREFIX = "\0vize:";
const VIRTUAL_CSS_MODULE = "virtual:vize-styles";
const RESOLVED_CSS_MODULE = "\0vize:all-styles.css";

interface DynamicImportAliasRule {
  fromPrefix: string;
  toPrefix: string;
}

/** Check if a module ID is a vize-compiled virtual module */
function isVizeVirtual(id: string): boolean {
  return id.startsWith("\0") && id.endsWith(".vue.ts");
}

/** Create a virtual module ID from a real .vue file path */
function toVirtualId(realPath: string): string {
  return "\0" + realPath + ".ts";
}

/** Extract the real .vue file path from a virtual module ID */
function fromVirtualId(virtualId: string): string {
  // Strip \0 prefix and .ts suffix
  return virtualId.slice(1, -3);
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function toBrowserImportPrefix(replacement: string): string {
  const normalized = replacement.replace(/\\/g, "/");
  if (normalized.startsWith("/@fs/")) {
    return normalized;
  }
  // Absolute filesystem alias targets should be served via /@fs in browser imports.
  if (path.isAbsolute(replacement) && fs.existsSync(replacement)) {
    return `/@fs${normalized}`;
  }
  return normalized;
}

function normalizeFsIdForBuild(id: string): string {
  const [pathPart, queryPart] = id.split("?");
  if (!pathPart.startsWith("/@fs/")) {
    return id;
  }
  const normalizedPath = pathPart.slice(4); // strip '/@fs'
  return queryPart ? `${normalizedPath}?${queryPart}` : normalizedPath;
}

function rewriteDynamicTemplateImports(code: string, aliasRules: DynamicImportAliasRule[]): string {
  let rewritten = code;

  // Normalize alias-based template literal imports (e.g. `@/foo/${x}.svg`) to browser paths.
  for (const rule of aliasRules) {
    const pattern = new RegExp(`\\bimport\\s*\\(\\s*\`${escapeRegExp(rule.fromPrefix)}`, "g");
    rewritten = rewritten.replace(pattern, `import(/* @vite-ignore */ \`${rule.toPrefix}`);
  }

  // Dynamic template imports are intentionally runtime-resolved: mark them to silence
  // Vite's static analysis warning while keeping runtime behavior.
  rewritten = rewritten.replace(/\bimport\s*\(\s*`/g, "import(/* @vite-ignore */ `");

  return rewritten;
}

/**
 * Rewrite static asset URLs in compiled template output.
 *
 * Transforms property values like `src: "@/assets/logo.svg"` into import
 * statements hoisted to the top of the module, so Vite's module resolution
 * pipeline handles alias expansion and asset hashing in both dev and build.
 */
function rewriteStaticAssetUrls(code: string, aliasRules: DynamicImportAliasRule[]): string {
  let rewritten = code;
  const imports: string[] = [];
  let counter = 0;

  for (const rule of aliasRules) {
    // Match patterns:
    //   src: "@/..."  or  "src": "@/..."  (double quotes)
    //   src: '@/...'  or  "src": '@/...'  (single quotes)
    const pattern = new RegExp(
      `("?src"?\\s*:\\s*)(?:"(${escapeRegExp(rule.fromPrefix)}[^"]+)"|'(${escapeRegExp(rule.fromPrefix)}[^']+)')`,
      "g",
    );
    rewritten = rewritten.replace(
      pattern,
      (_match: string, prefix: string, dqPath?: string, sqPath?: string) => {
        const fullPath = dqPath || sqPath;
        const varName = `__vize_static_${counter++}`;
        imports.push(`import ${varName} from ${JSON.stringify(fullPath)};`);
        return `${prefix}${varName}`;
      },
    );
  }

  if (imports.length > 0) {
    rewritten = imports.join("\n") + "\n" + rewritten;
  }
  return rewritten;
}

// Test-only export for snapshot coverage.
export const __internal = {
  rewriteStaticAssetUrls,
};

function createLogger(debug: boolean) {
  return {
    log: (...args: unknown[]) => debug && console.log("[vize]", ...args),
    info: (...args: unknown[]) => console.log("[vize]", ...args),
    warn: (...args: unknown[]) => console.warn("[vize]", ...args),
    error: (...args: unknown[]) => console.error("[vize]", ...args),
  };
}

export function vize(options: VizeOptions = {}): Plugin[] {
  const cache = new Map<string, CompiledModule>();
  // Collected CSS for production extraction
  const collectedCss = new Map<string, string>();

  let isProduction: boolean;
  let root: string;
  let server: ViteDevServer | null = null;
  let filter: (id: string) => boolean;
  let scanPatterns: string[];
  let ignorePatterns: string[];
  let mergedOptions: VizeOptions;
  let dynamicImportAliasRules: DynamicImportAliasRule[] = [];
  let extractCss = false;

  const logger = createLogger(options.debug ?? false);

  async function compileAll(): Promise<void> {
    const startTime = performance.now();
    const files = await glob(scanPatterns, {
      cwd: root,
      ignore: ignorePatterns,
      absolute: true,
    });

    logger.info(`Pre-compiling ${files.length} Vue files...`);

    // Read all files
    const fileContents: { path: string; source: string }[] = [];
    for (const file of files) {
      try {
        const source = fs.readFileSync(file, "utf-8");
        fileContents.push({ path: file, source });
      } catch (e) {
        logger.error(`Failed to read ${file}:`, e);
      }
    }

    // Batch compile using native parallel processing
    const result = compileBatch(fileContents, cache, {
      ssr: mergedOptions.ssr ?? false,
    });

    // Collect CSS for production extraction
    if (isProduction) {
      for (const fileResult of result.results) {
        if (fileResult.css) {
          collectedCss.set(fileResult.path, fileResult.css);
        }
      }
    }

    const elapsed = (performance.now() - startTime).toFixed(2);
    logger.info(
      `Pre-compilation complete: ${result.successCount} succeeded, ${result.failedCount} failed (${elapsed}ms, native batch: ${result.timeMs.toFixed(2)}ms)`,
    );
  }

  function resolveVuePath(id: string, importer?: string): string {
    let resolved: string;
    // Handle Vite's /@fs/ prefix for absolute filesystem paths
    if (id.startsWith("/@fs/")) {
      resolved = id.slice(4); // Remove '/@fs' prefix, keep the absolute path
    } else if (id.startsWith("/") && !fs.existsSync(id)) {
      // Check if it's a web-root relative path (starts with / but not a real absolute path)
      // These are relative to the project root, not the filesystem root
      // Remove leading slash and resolve relative to root
      resolved = path.resolve(root, id.slice(1));
    } else if (path.isAbsolute(id)) {
      resolved = id;
    } else if (importer) {
      // If importer is a virtual module, extract the real path
      const realImporter = isVizeVirtual(importer) ? fromVirtualId(importer) : importer;
      resolved = path.resolve(path.dirname(realImporter), id);
    } else {
      // Relative path without importer - resolve from root
      resolved = path.resolve(root, id);
    }
    // Ensure we always return an absolute path
    if (!path.isAbsolute(resolved)) {
      resolved = path.resolve(root, resolved);
    }
    return path.normalize(resolved);
  }

  const mainPlugin: Plugin = {
    name: "vite-plugin-vize",
    enforce: "pre",

    config(_, env) {
      return {
        // Vue 3 ESM bundler build requires these compile-time feature flags.
        // @vitejs/plugin-vue normally provides them; vize must do so as its replacement.
        define: {
          __VUE_OPTIONS_API__: true,
          __VUE_PROD_DEVTOOLS__: env.command === "serve",
          __VUE_PROD_HYDRATION_MISMATCH_DETAILS__: false,
        },
        optimizeDeps: {
          include: ["vue"],
          exclude: ["virtual:vize-styles"],
          esbuildOptions: {
            plugins: [
              {
                name: "vize-externalize-vue",
                setup(build) {
                  build.onResolve({ filter: /\.vue$/ }, (args) => ({
                    path: args.path,
                    external: true,
                  }));
                },
              },
            ],
          },
          rolldownOptions: {
            external: [/\.vue$/],
          },
        },
      };
    },

    async configResolved(resolvedConfig: ResolvedConfig) {
      root = options.root ?? resolvedConfig.root;
      isProduction = options.isProduction ?? resolvedConfig.isProduction;
      extractCss = isProduction;

      const configEnv: ConfigEnv = {
        mode: resolvedConfig.mode,
        command: resolvedConfig.command === "build" ? "build" : "serve",
        isSsrBuild: !!resolvedConfig.build?.ssr,
      };

      let fileConfig: VizeConfig | null = null;
      if (options.configMode !== false) {
        fileConfig = await loadConfig(root, {
          mode: options.configMode ?? "root",
          configFile: options.configFile,
          env: configEnv,
        });
        if (fileConfig) {
          logger.log("Loaded config from vize.config file");
          vizeConfigStore.set(root, fileConfig);
        }
      }

      const viteConfig = fileConfig?.vite ?? {};
      const compilerConfig = fileConfig?.compiler ?? {};

      mergedOptions = {
        ...options,
        ssr: options.ssr ?? compilerConfig.ssr ?? false,
        sourceMap: options.sourceMap ?? compilerConfig.sourceMap,
        vapor: options.vapor ?? compilerConfig.vapor ?? false,
        include: options.include ?? viteConfig.include,
        exclude: options.exclude ?? viteConfig.exclude,
        scanPatterns: options.scanPatterns ?? viteConfig.scanPatterns,
        ignorePatterns: options.ignorePatterns ?? viteConfig.ignorePatterns,
      };

      dynamicImportAliasRules = [];
      for (const alias of resolvedConfig.resolve.alias) {
        if (typeof alias.find !== "string" || typeof alias.replacement !== "string") {
          continue;
        }
        const fromPrefix = alias.find.endsWith("/") ? alias.find : `${alias.find}/`;
        const replacement = toBrowserImportPrefix(alias.replacement);
        const toPrefix = replacement.endsWith("/") ? replacement : `${replacement}/`;
        dynamicImportAliasRules.push({ fromPrefix, toPrefix });
      }
      // Prefer longer alias keys first (e.g. "@@" before "@")
      dynamicImportAliasRules.sort((a, b) => b.fromPrefix.length - a.fromPrefix.length);

      filter = createFilter(mergedOptions.include, mergedOptions.exclude);
      scanPatterns = mergedOptions.scanPatterns ?? ["**/*.vue"];
      ignorePatterns = mergedOptions.ignorePatterns ?? ["node_modules/**", "dist/**", ".git/**"];
    },

    configureServer(devServer: ViteDevServer) {
      server = devServer;

      // Rewrite __x00__ URLs from virtual module dynamic imports.
      // When compiled .vue files contain dynamic imports (e.g., template literal imports
      // for SVGs), the browser resolves them relative to the virtual module URL which
      // contains \0 (encoded as __x00__). Vite's plugin container short-circuits
      // resolveId for \0-prefixed IDs, so we intercept at the middleware level and
      // rewrite to /@fs/ so Vite serves the actual file.
      devServer.middlewares.use((req, _res, next) => {
        if (req.url && req.url.includes("__x00__")) {
          const [urlPath, queryPart] = req.url.split("?");
          // e.g., /@id/__x00__/Users/.../help.svg?import → /@fs/Users/.../help.svg?import
          let cleanedPath = urlPath.replace(/__x00__/g, "");
          // After removing __x00__, /@id//Users/... has double slash — normalize to /@fs/
          cleanedPath = cleanedPath.replace(/^\/@id\/\//, "/@fs/");

          // Do not rewrite vize virtual Vue modules (e.g. /@id/__x00__/.../App.vue.ts),
          // they must go through plugin load() and are not real files on disk.
          if (cleanedPath.startsWith("/@fs/")) {
            const fsPath = cleanedPath.slice(4); // strip '/@fs'
            if (
              fsPath.startsWith("/") &&
              fs.existsSync(fsPath) &&
              fs.statSync(fsPath).isFile() &&
              !fsPath.endsWith(".vue.ts")
            ) {
              const cleaned = queryPart ? `${cleanedPath}?${queryPart}` : cleanedPath;
              if (cleaned !== req.url) {
                logger.log(`middleware: rewriting ${req.url} → ${cleaned}`);
                req.url = cleaned;
              }
            }
          }
        }
        next();
      });
    },

    async buildStart() {
      await compileAll();
      logger.log("Cache keys:", [...cache.keys()].slice(0, 3));
    },

    async resolveId(id: string, importer?: string) {
      const isBuild = server === null;

      // Skip all virtual module IDs
      if (id.startsWith("\0")) {
        // This is one of our .vue.ts virtual modules — pass through
        if (isVizeVirtual(id)) {
          return null;
        }
        // Legacy: handle old \0vize: prefixed non-vue files
        if (id.startsWith(LEGACY_VIZE_PREFIX)) {
          const rawPath = id.slice(LEGACY_VIZE_PREFIX.length);
          const cleanPath = rawPath.endsWith(".ts") ? rawPath.slice(0, -3) : rawPath;
          if (!cleanPath.endsWith(".vue")) {
            logger.log(`resolveId: redirecting legacy virtual ID to ${cleanPath}`);
            return cleanPath;
          }
        }
        // Redirect non-vue files that accidentally got \0 prefix.
        // This happens when Vite's import analysis resolves dynamic imports
        // relative to virtual module paths — the \0 prefix leaks into the
        // resolved path and appears as __x00__ in browser URLs.
        const cleanPath = id.slice(1); // strip \0
        if (cleanPath.startsWith("/") && !cleanPath.endsWith(".vue.ts")) {
          // Strip query params for existence check
          const [pathPart, queryPart] = cleanPath.split("?");
          const querySuffix = queryPart ? `?${queryPart}` : "";
          logger.log(`resolveId: redirecting \0-prefixed non-vue ID to ${pathPart}${querySuffix}`);
          const redirected = pathPart + querySuffix;
          return isBuild ? normalizeFsIdForBuild(redirected) : redirected;
        }
        return null;
      }

      // Handle stale vize: prefix (without \0) from cached resolutions
      if (id.startsWith("vize:")) {
        let realPath = id.slice("vize:".length);
        if (realPath.endsWith(".ts")) {
          realPath = realPath.slice(0, -3);
        }
        logger.log(`resolveId: redirecting stale vize: ID to ${realPath}`);
        const resolved = await this.resolve(realPath, importer, { skipSelf: true });
        if (resolved && isBuild && resolved.id.startsWith("/@fs/")) {
          return { ...resolved, id: normalizeFsIdForBuild(resolved.id) };
        }
        return resolved;
      }

      // Handle virtual CSS module for production extraction
      if (id === VIRTUAL_CSS_MODULE) {
        return RESOLVED_CSS_MODULE;
      }

      if (isBuild && id.startsWith("/@fs/")) {
        return normalizeFsIdForBuild(id);
      }

      if (id.includes("?vue&type=style")) {
        return id;
      }

      // If importer is a vize virtual module, resolve non-vue imports against the real path
      if (importer && isVizeVirtual(importer)) {
        const cleanImporter = fromVirtualId(importer);

        logger.log(`resolveId from virtual: id=${id}, cleanImporter=${cleanImporter}`);

        // Subpath imports (e.g., #imports/entry from Nuxt)
        if (id.startsWith("#")) {
          try {
            return await this.resolve(id, cleanImporter, { skipSelf: true });
          } catch {
            return null;
          }
        }

        // For non-vue files, resolve relative to the real importer
        if (!id.endsWith(".vue")) {
          if (id.includes("/dist/") || id.includes("/lib/") || id.includes("/es/")) {
            return null;
          }

          // Delegate to Vite's full resolver pipeline with the real importer
          try {
            const resolved = await this.resolve(id, cleanImporter, { skipSelf: true });
            if (resolved) {
              logger.log(`resolveId: resolved ${id} to ${resolved.id} via Vite resolver`);
              if (isBuild && resolved.id.startsWith("/@fs/")) {
                return { ...resolved, id: normalizeFsIdForBuild(resolved.id) };
              }
              return resolved;
            }
          } catch {
            // Fall through to manual resolution
          }

          // Fallback: manual resolution for relative imports
          if (id.startsWith("./") || id.startsWith("../")) {
            const [pathPart, queryPart] = id.split("?");
            const querySuffix = queryPart ? `?${queryPart}` : "";

            const resolved = path.resolve(path.dirname(cleanImporter), pathPart);
            for (const ext of ["", ".ts", ".tsx", ".js", ".jsx", ".json"]) {
              const candidate = resolved + ext;
              if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
                const finalPath = candidate + querySuffix;
                logger.log(`resolveId: resolved relative ${id} to ${finalPath}`);
                return finalPath;
              }
            }
            if (fs.existsSync(resolved) && fs.statSync(resolved).isDirectory()) {
              for (const indexFile of ["/index.ts", "/index.tsx", "/index.js", "/index.jsx"]) {
                const candidate = resolved + indexFile;
                if (fs.existsSync(candidate)) {
                  const finalPath = candidate + querySuffix;
                  logger.log(`resolveId: resolved directory ${id} to ${finalPath}`);
                  return finalPath;
                }
              }
            }
          }

          return null;
        }
      }

      // Handle .vue file imports
      if (id.endsWith(".vue")) {
        const handleNodeModules = mergedOptions.handleNodeModulesVue ?? true;

        if (!handleNodeModules && id.includes("node_modules")) {
          logger.log(`resolveId: skipping node_modules import ${id}`);
          return null;
        }

        const resolved = resolveVuePath(id, importer);
        const isNodeModulesPath = resolved.includes("node_modules");

        if (!handleNodeModules && isNodeModulesPath) {
          logger.log(`resolveId: skipping node_modules path ${resolved}`);
          return null;
        }

        if (!isNodeModulesPath && !filter(resolved)) {
          logger.log(`resolveId: skipping filtered path ${resolved}`);
          return null;
        }

        const hasCache = cache.has(resolved);
        const fileExists = fs.existsSync(resolved);
        logger.log(
          `resolveId: id=${id}, resolved=${resolved}, hasCache=${hasCache}, fileExists=${fileExists}, importer=${importer ?? "none"}`,
        );

        // Return virtual module ID: \0/path/to/Component.vue.ts
        if (hasCache || fileExists) {
          return toVirtualId(resolved);
        }

        // Vite fallback for aliased imports
        if (!fileExists && !path.isAbsolute(id)) {
          const viteResolved = await this.resolve(id, importer, { skipSelf: true });
          if (viteResolved && viteResolved.id.endsWith(".vue")) {
            const realPath = viteResolved.id;
            const isResolvedNodeModules = realPath.includes("node_modules");
            if (
              (isResolvedNodeModules ? handleNodeModules : filter(realPath)) &&
              (cache.has(realPath) || fs.existsSync(realPath))
            ) {
              logger.log(`resolveId: resolved via Vite fallback ${id} to ${realPath}`);
              return toVirtualId(realPath);
            }
          }
        }
      }

      return null;
    },

    load(id: string) {
      // Handle virtual CSS module for production extraction
      if (id === RESOLVED_CSS_MODULE) {
        const allCss = Array.from(collectedCss.values()).join("\n\n");
        return allCss;
      }

      if (id.includes("?vue&type=style")) {
        const [filename] = id.split("?");
        // Extract real path from virtual ID or use as-is
        const realPath = isVizeVirtual(filename) ? fromVirtualId(filename) : filename;
        const compiled = cache.get(realPath);
        if (compiled?.css) {
          return compiled.css;
        }
        return "";
      }

      // Handle vize virtual modules
      if (isVizeVirtual(id)) {
        const realPath = fromVirtualId(id);

        if (!realPath.endsWith(".vue")) {
          logger.log(`load: skipping non-vue virtual module ${realPath}`);
          return null;
        }

        let compiled = cache.get(realPath);

        // On-demand compile if not cached
        if (!compiled && fs.existsSync(realPath)) {
          logger.log(`load: on-demand compiling ${realPath}`);
          compiled = compileFile(realPath, cache, {
            sourceMap: mergedOptions.sourceMap ?? !isProduction,
            ssr: mergedOptions.ssr ?? false,
          });
        }

        if (compiled) {
          const output = rewriteStaticAssetUrls(
            rewriteDynamicTemplateImports(
              generateOutput(compiled, {
                isProduction,
                isDev: server !== null,
                extractCss,
              }),
              dynamicImportAliasRules,
            ),
            dynamicImportAliasRules,
          );
          return {
            code: output,
            map: null,
          };
        }
      }

      // Handle \0-prefixed non-vue files leaked from virtual module dynamic imports.
      // When compiled .vue modules contain dynamic imports (e.g., template literal SVG imports),
      // Vite resolves them relative to the virtual module path, prepending \0 to the result.
      // The plugin container short-circuits resolveId for \0-prefixed IDs, so we intercept
      // in load and return a proxy module that re-exports from the real filesystem path.
      if (id.startsWith("\0")) {
        const afterPrefix = id.startsWith(LEGACY_VIZE_PREFIX)
          ? id.slice(LEGACY_VIZE_PREFIX.length)
          : id.slice(1);
        // Let @rollup/plugin-commonjs handle its own virtual IDs.
        if (afterPrefix.includes("?commonjs-")) {
          return null;
        }
        const [pathPart, queryPart] = afterPrefix.split("?");
        const querySuffix = queryPart ? `?${queryPart}` : "";
        const fsPath = pathPart.startsWith("/@fs/") ? pathPart.slice(4) : pathPart;
        if (fsPath.startsWith("/") && fs.existsSync(fsPath) && fs.statSync(fsPath).isFile()) {
          const importPath =
            server === null
              ? `${pathToFileURL(fsPath).href}${querySuffix}`
              : "/@fs" + fsPath + querySuffix;
          logger.log(`load: proxying \0-prefixed file ${id} → re-export from ${importPath}`);
          return `export { default } from ${JSON.stringify(importPath)};\nexport * from ${JSON.stringify(importPath)};`;
        }
      }

      return null;
    },

    // Strip TypeScript from compiled .vue output
    async transform(code: string, id: string): Promise<TransformResult | null> {
      if (isVizeVirtual(id)) {
        const realPath = fromVirtualId(id);
        try {
          const result = await transformWithOxc(code, realPath, {
            lang: "ts",
          });
          return { code: result.code, map: result.map as TransformResult["map"] };
        } catch (e: unknown) {
          logger.error(`transformWithOxc failed for ${realPath}:`, e);
          const dumpPath = `/tmp/vize-oxc-error-${path.basename(realPath)}.ts`;
          fs.writeFileSync(dumpPath, code, "utf-8");
          logger.error(`Dumped failing code to ${dumpPath}`);
          return { code: "export default {}", map: null };
        }
      }
      return null;
    },

    async handleHotUpdate(ctx: HmrContext) {
      const { file, server, read } = ctx;

      if (file.endsWith(".vue") && filter(file)) {
        try {
          const source = await read();

          const prevCompiled = cache.get(file);

          compileFile(
            file,
            cache,
            {
              sourceMap: mergedOptions.sourceMap ?? !isProduction,
              ssr: mergedOptions.ssr ?? false,
            },
            source,
          );

          const newCompiled = cache.get(file)!;

          const updateType: HmrUpdateType = detectHmrUpdateType(prevCompiled, newCompiled);

          logger.log(`Re-compiled: ${path.relative(root, file)} (${updateType})`);

          // Find module by virtual ID or real file path
          const virtualId = toVirtualId(file);
          const modules =
            server.moduleGraph.getModulesByFile(virtualId) ??
            server.moduleGraph.getModulesByFile(file);

          // For style-only updates, send custom event
          if (updateType === "style-only" && newCompiled.css) {
            server.ws.send({
              type: "custom",
              event: "vize:update",
              data: {
                id: newCompiled.scopeId,
                type: "style-only",
                css: newCompiled.css,
              },
            });
            return [];
          }

          if (modules) {
            return [...modules];
          }
        } catch (e) {
          logger.error(`Re-compilation failed for ${file}:`, e);
        }
      }
    },

    generateBundle(_, _bundle) {
      if (!extractCss || collectedCss.size === 0) {
        return;
      }

      const allCss = Array.from(collectedCss.values()).join("\n\n");
      if (allCss.trim()) {
        this.emitFile({
          type: "asset",
          fileName: "assets/vize-components.css",
          source: allCss,
        });
        logger.log(`Extracted CSS to assets/vize-components.css (${collectedCss.size} components)`);
      }
    },
  };

  let compilerSfc: unknown = null;
  const loadCompilerSfc = () => {
    if (!compilerSfc) {
      try {
        const require = createRequire(import.meta.url);
        compilerSfc = require("@vue/compiler-sfc");
      } catch {
        compilerSfc = { parse: () => ({ descriptor: {}, errors: [] }) };
      }
    }
    return compilerSfc;
  };
  const vueCompatPlugin: Plugin = {
    name: "vite:vue",
    api: {
      get options() {
        return {
          compiler: loadCompilerSfc(),
          isProduction: isProduction ?? false,
          root: root ?? process.cwd(),
          template: {},
        };
      },
    },
  };

  return [vueCompatPlugin, mainPlugin];
}

export default vize;
