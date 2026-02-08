import type { Plugin, ResolvedConfig, ViteDevServer, HmrContext, TransformResult } from "vite";
import { transformWithOxc } from "vite";
import path from "node:path";
import fs from "node:fs";
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

const VIRTUAL_PREFIX = "\0vize:";
const VIRTUAL_CSS_MODULE = "virtual:vize-styles";
const RESOLVED_CSS_MODULE = "\0vize:all-styles.css";

function createLogger(debug: boolean) {
  return {
    log: (...args: unknown[]) => debug && console.log("[vize]", ...args),
    info: (...args: unknown[]) => console.log("[vize]", ...args),
    warn: (...args: unknown[]) => console.warn("[vize]", ...args),
    error: (...args: unknown[]) => console.error("[vize]", ...args),
  };
}

export function vize(options: VizeOptions = {}): Plugin {
  const cache = new Map<string, CompiledModule>();
  // Map from virtual ID to real file path
  const virtualToReal = new Map<string, string>();
  // Collected CSS for production extraction
  const collectedCss = new Map<string, string>();

  let isProduction: boolean;
  let root: string;
  let server: ViteDevServer | null = null;
  let filter: (id: string) => boolean;
  let scanPatterns: string[];
  let ignorePatterns: string[];
  let mergedOptions: VizeOptions;
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
      // Remove virtual prefix from importer if present
      let realImporter = importer.startsWith(VIRTUAL_PREFIX)
        ? (virtualToReal.get(importer) ?? importer.slice(VIRTUAL_PREFIX.length))
        : importer;
      // Remove .ts suffix that we add to virtual IDs
      if (realImporter.endsWith(".vue.ts")) {
        realImporter = realImporter.slice(0, -3);
      }
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

  return {
    name: "vite-plugin-vize",
    enforce: "pre",

    config() {
      // Exclude virtual modules and .vue files from dependency optimization
      // Vize resolves .vue files to virtual modules with \0 prefix,
      // which causes esbuild (Vite 6) / rolldown (Vite 8) dep scanning to fail
      // because they try to read the \0-prefixed path as a real file.
      return {
        optimizeDeps: {
          // Ensure vue is always pre-optimized so dep scan failures
          // for .vue virtual modules don't cause mid-serve reloads
          include: ["vue"],
          exclude: ["virtual:vize-styles"],
          // Vite 6: prevent esbuild dep scanner from processing .vue files
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
          // Vite 8: prevent rolldown from processing .vue files
          rolldownOptions: {
            external: [/\.vue$/],
          },
        },
      };
    },

    async configResolved(resolvedConfig: ResolvedConfig) {
      root = options.root ?? resolvedConfig.root;
      isProduction = options.isProduction ?? resolvedConfig.isProduction;
      extractCss = isProduction; // Extract CSS in production by default

      // Build ConfigEnv for dynamic config resolution
      const configEnv: ConfigEnv = {
        mode: resolvedConfig.mode,
        command: resolvedConfig.command === "build" ? "build" : "serve",
        isSsrBuild: !!resolvedConfig.build?.ssr,
      };

      // Load config file if enabled
      let fileConfig: VizeConfig | null = null;
      if (options.configMode !== false) {
        fileConfig = await loadConfig(root, {
          mode: options.configMode ?? "root",
          configFile: options.configFile,
          env: configEnv,
        });
        if (fileConfig) {
          logger.log("Loaded config from vize.config file");
          // Store in shared config store for other plugins (e.g. musea)
          vizeConfigStore.set(root, fileConfig);
        }
      }

      // Merge options: plugin options > config file > defaults
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

      filter = createFilter(mergedOptions.include, mergedOptions.exclude);
      scanPatterns = mergedOptions.scanPatterns ?? ["**/*.vue"];
      ignorePatterns = mergedOptions.ignorePatterns ?? ["node_modules/**", "dist/**", ".git/**"];
    },

    configureServer(devServer: ViteDevServer) {
      server = devServer;
    },

    async buildStart() {
      await compileAll();
      // Debug: log cache keys
      logger.log("Cache keys:", [...cache.keys()].slice(0, 3));
    },

    async resolveId(id: string, importer?: string) {
      // Skip virtual module IDs starting with \0
      if (id.startsWith("\0")) {
        return null;
      }

      // Handle stale vize: prefix (without \0) from cached resolutions
      // Redirect to original file path so Vite/other plugins can handle it
      if (id.startsWith("vize:")) {
        let realPath = id.slice("vize:".length);
        if (realPath.endsWith(".ts")) {
          realPath = realPath.slice(0, -3);
        }
        logger.log(`resolveId: redirecting stale vize: ID to ${realPath}`);
        // For node_modules, return the original path to let Vite handle it normally
        if (realPath.includes("node_modules")) {
          return realPath;
        }
        // For project files, resolve through vize again
        return this.resolve(realPath, importer, { skipSelf: true });
      }

      // Handle virtual CSS module for production extraction
      if (id === VIRTUAL_CSS_MODULE) {
        return RESOLVED_CSS_MODULE;
      }

      if (id.includes("?vue&type=style")) {
        return id;
      }

      // If importer is a virtual module, resolve imports against the real path
      if (importer?.startsWith(VIRTUAL_PREFIX)) {
        const realImporter = virtualToReal.get(importer) ?? importer.slice(VIRTUAL_PREFIX.length);
        // Remove .ts suffix if present
        const cleanImporter = realImporter.endsWith(".ts")
          ? realImporter.slice(0, -3)
          : realImporter;

        logger.log(`resolveId from virtual: id=${id}, cleanImporter=${cleanImporter}`);

        // For non-vue files, resolve relative to the real importer
        if (!id.endsWith(".vue")) {
          if (id.startsWith("./") || id.startsWith("../")) {
            // Separate query params (e.g., ?inline, ?raw) from the path
            const [pathPart, queryPart] = id.split("?");
            const querySuffix = queryPart ? `?${queryPart}` : "";

            // Relative imports - resolve and check if file exists
            const resolved = path.resolve(path.dirname(cleanImporter), pathPart);
            for (const ext of ["", ".ts", ".tsx", ".js", ".jsx", ".json"]) {
              if (fs.existsSync(resolved + ext)) {
                const finalPath = resolved + ext + querySuffix;
                logger.log(`resolveId: resolved relative ${id} to ${finalPath}`);
                return finalPath;
              }
            }
          } else {
            // External package imports (e.g., '@mdi/js', 'vue')
            // Check if the id looks like an already-resolved path (contains /dist/ or /lib/)
            // This can happen when other plugins (like vue-i18n) have already transformed the import
            if (id.includes("/dist/") || id.includes("/lib/") || id.includes("/es/")) {
              // Already looks resolved, return null to let Vite handle it
              logger.log(`resolveId: skipping already-resolved path ${id}`);
              return null;
            }
            // Re-resolve with the real importer path
            logger.log(`resolveId: resolving external ${id} from ${cleanImporter}`);
            const resolved = await this.resolve(id, cleanImporter, {
              skipSelf: true,
            });
            logger.log(`resolveId: resolved external ${id} to`, resolved?.id ?? "null");
            return resolved;
          }
        }
      }

      if (id.endsWith(".vue")) {
        // Skip node_modules early - before even resolving the path
        // This handles cases where the import path itself contains node_modules
        if (id.includes("node_modules")) {
          logger.log(`resolveId: skipping node_modules import ${id}`);
          return null;
        }

        const resolved = resolveVuePath(id, importer);

        // Skip node_modules - frameworks like Nuxt have their own Vue plugins
        // This must be checked BEFORE any caching or virtual ID creation
        if (resolved.includes("node_modules")) {
          logger.log(`resolveId: skipping node_modules path ${resolved}`);
          return null;
        }

        // Skip if not matching filter (additional user-configured exclusions)
        if (!filter(resolved)) {
          logger.log(`resolveId: skipping filtered path ${resolved}`);
          return null;
        }

        // Debug: log all resolution attempts
        const hasCache = cache.has(resolved);
        const fileExists = fs.existsSync(resolved);
        logger.log(
          `resolveId: id=${id}, resolved=${resolved}, hasCache=${hasCache}, fileExists=${fileExists}, importer=${importer ?? "none"}`,
        );

        // Return virtual module ID if cached or file exists
        // Add .ts suffix so Vite transforms TypeScript
        // If not in cache, the load hook will compile on-demand
        if (hasCache || fileExists) {
          const virtualId = VIRTUAL_PREFIX + resolved + ".ts";
          virtualToReal.set(virtualId, resolved);
          return virtualId;
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
        const realPath = filename.startsWith(VIRTUAL_PREFIX)
          ? (virtualToReal.get(filename) ?? filename.slice(VIRTUAL_PREFIX.length))
          : filename;
        const compiled = cache.get(realPath);
        if (compiled?.css) {
          return compiled.css;
        }
        return "";
      }

      if (id.startsWith(VIRTUAL_PREFIX)) {
        // Remove .ts suffix if present for lookup
        const lookupId = id.endsWith(".ts") ? id.slice(0, -3) : id;
        const realPath = virtualToReal.get(id) ?? lookupId.slice(VIRTUAL_PREFIX.length);
        const compiled = cache.get(realPath);

        if (compiled) {
          const output = generateOutput(compiled, {
            isProduction,
            isDev: server !== null,
            extractCss,
          });
          return {
            code: output,
            map: null,
          };
        }
      }

      return null;
    },

    // Transform virtual modules: strip TypeScript since \0-prefixed virtual modules
    // bypass Vite's built-in transform plugins
    async transform(code: string, id: string): Promise<TransformResult | null> {
      if (id.startsWith(VIRTUAL_PREFIX) && id.endsWith(".ts")) {
        const result = await transformWithOxc(code, id.slice(VIRTUAL_PREFIX.length), {
          lang: "ts",
        });
        return { code: result.code, map: result.map };
      }
      return null;
    },

    async handleHotUpdate(ctx: HmrContext) {
      const { file, server, read } = ctx;

      if (file.endsWith(".vue") && filter(file)) {
        try {
          const source = await read();

          // Get previous compiled module for change detection
          const prevCompiled = cache.get(file);

          // Recompile
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

          // Detect HMR update type
          const updateType: HmrUpdateType = detectHmrUpdateType(prevCompiled, newCompiled);

          logger.log(`Re-compiled: ${path.relative(root, file)} (${updateType})`);

          // Find the virtual module for this file
          const virtualId = VIRTUAL_PREFIX + file + ".ts";
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
            // Return empty array to prevent full module reload
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

    // Production CSS extraction
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
}

export default vize;
