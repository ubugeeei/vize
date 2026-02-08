import * as fs from "node:fs";
import * as path from "node:path";
import { pathToFileURL } from "node:url";
import { transform } from "oxc-transform";
import type {
  VizeConfig,
  LoadConfigOptions,
  UserConfigExport,
  ConfigEnv,
  GlobalTypesConfig,
  GlobalTypeDeclaration,
} from "./types.js";

const CONFIG_FILE_NAMES = [
  "vize.config.ts",
  "vize.config.js",
  "vize.config.mjs",
  "vize.config.json",
];

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
 * Load vize.config file from the specified directory
 */
export async function loadConfig(
  root: string,
  options: LoadConfigOptions = {},
): Promise<VizeConfig | null> {
  const { mode = "root", configFile, env } = options;

  if (mode === "none") {
    return null;
  }

  // Custom config file path
  if (configFile) {
    const absolutePath = path.isAbsolute(configFile) ? configFile : path.resolve(root, configFile);
    if (fs.existsSync(absolutePath)) {
      return loadConfigFile(absolutePath, env);
    }
    return null;
  }

  // Search for config file
  if (mode === "auto") {
    const configPath = findConfigFileAuto(root);
    if (!configPath) {
      return null;
    }
    return loadConfigFile(configPath, env);
  }

  // mode === "root"
  const configPath = findConfigFileInDir(root);
  if (!configPath) {
    return null;
  }
  return loadConfigFile(configPath, env);
}

/**
 * Find config file in a specific directory
 */
function findConfigFileInDir(dir: string): string | null {
  for (const name of CONFIG_FILE_NAMES) {
    const filePath = path.join(dir, name);
    if (fs.existsSync(filePath)) {
      return filePath;
    }
  }
  return null;
}

/**
 * Find config file by searching from cwd upward
 */
function findConfigFileAuto(startDir: string): string | null {
  let currentDir = path.resolve(startDir);
  const root = path.parse(currentDir).root;

  while (currentDir !== root) {
    const configPath = findConfigFileInDir(currentDir);
    if (configPath) {
      return configPath;
    }
    currentDir = path.dirname(currentDir);
  }

  return null;
}

/**
 * Load and evaluate a config file
 */
async function loadConfigFile(filePath: string, env?: ConfigEnv): Promise<VizeConfig | null> {
  if (!fs.existsSync(filePath)) {
    return null;
  }

  const ext = path.extname(filePath);

  if (ext === ".json") {
    const content = fs.readFileSync(filePath, "utf-8");
    return JSON.parse(content);
  }

  if (ext === ".ts") {
    return loadTypeScriptConfig(filePath, env);
  }

  // .js, .mjs - ESM
  return loadESMConfig(filePath, env);
}

/**
 * Resolve a UserConfigExport to a VizeConfig
 */
async function resolveConfigExport(
  exported: UserConfigExport,
  env?: ConfigEnv,
): Promise<VizeConfig> {
  if (typeof exported === "function") {
    return exported(env ?? DEFAULT_CONFIG_ENV);
  }
  return exported;
}

/**
 * Load TypeScript config file using oxc-transform
 */
async function loadTypeScriptConfig(filePath: string, env?: ConfigEnv): Promise<VizeConfig> {
  const source = fs.readFileSync(filePath, "utf-8");
  const result = transform(filePath, source, {
    typescript: {
      onlyRemoveTypeImports: true,
    },
  });

  const code = result.code;

  // Write to temp file and import (use Date.now() to avoid race conditions)
  const tempFile = filePath.replace(/\.ts$/, `.temp.${Date.now()}.mjs`);
  fs.writeFileSync(tempFile, code);

  try {
    const fileUrl = pathToFileURL(tempFile).href;
    const module = await import(fileUrl);
    const exported: UserConfigExport = module.default || module;
    return resolveConfigExport(exported, env);
  } finally {
    fs.unlinkSync(tempFile);
  }
}

/**
 * Load ESM config file
 */
async function loadESMConfig(filePath: string, env?: ConfigEnv): Promise<VizeConfig> {
  const fileUrl = pathToFileURL(filePath).href;
  const module = await import(fileUrl);
  const exported: UserConfigExport = module.default || module;
  return resolveConfigExport(exported, env);
}

/**
 * Normalize GlobalTypesConfig shorthand strings to GlobalTypeDeclaration objects
 */
export function normalizeGlobalTypes(
  config: GlobalTypesConfig,
): Record<string, GlobalTypeDeclaration> {
  const result: Record<string, GlobalTypeDeclaration> = {};
  for (const [key, value] of Object.entries(config)) {
    if (typeof value === "string") {
      result[key] = { type: value };
    } else {
      result[key] = value;
    }
  }
  return result;
}
