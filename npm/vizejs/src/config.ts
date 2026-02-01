import * as fs from 'node:fs'
import * as path from 'node:path'
import { pathToFileURL } from 'node:url'
import { transform } from 'oxc-transform'
import type { VizeConfig, LoadConfigOptions } from './types.js'

const CONFIG_FILE_NAMES = [
  'vize.config.ts',
  'vize.config.mts',
  'vize.config.js',
  'vize.config.mjs',
  'vize.config.cjs',
]

/**
 * Define vize configuration with type checking
 */
export function defineConfig(config: VizeConfig): VizeConfig {
  return config
}

/**
 * Load vize.config file from the specified directory
 */
export async function loadConfig(
  root: string,
  options: LoadConfigOptions = {}
): Promise<VizeConfig | null> {
  const { mode = 'root', configFile } = options

  // Custom config file path
  if (configFile) {
    const absolutePath = path.isAbsolute(configFile)
      ? configFile
      : path.resolve(root, configFile)
    if (fs.existsSync(absolutePath)) {
      return loadConfigFile(absolutePath)
    }
    return null
  }

  // Search for config file
  const configPath =
    mode === 'auto' ? findConfigFileAuto(root) : findConfigFileInDir(root)

  if (!configPath) {
    return null
  }

  return loadConfigFile(configPath)
}

/**
 * Find config file in a specific directory
 */
function findConfigFileInDir(dir: string): string | null {
  for (const name of CONFIG_FILE_NAMES) {
    const filePath = path.join(dir, name)
    if (fs.existsSync(filePath)) {
      return filePath
    }
  }
  return null
}

/**
 * Find config file by searching from cwd upward
 */
function findConfigFileAuto(startDir: string): string | null {
  let currentDir = path.resolve(startDir)
  const root = path.parse(currentDir).root

  while (currentDir !== root) {
    const configPath = findConfigFileInDir(currentDir)
    if (configPath) {
      return configPath
    }
    currentDir = path.dirname(currentDir)
  }

  return null
}

/**
 * Load and evaluate a config file
 */
async function loadConfigFile(filePath: string): Promise<VizeConfig> {
  const ext = path.extname(filePath)

  if (ext === '.ts' || ext === '.mts') {
    return loadTypeScriptConfig(filePath)
  }

  if (ext === '.cjs') {
    return loadCommonJSConfig(filePath)
  }

  // .js, .mjs - ESM
  return loadESMConfig(filePath)
}

/**
 * Load TypeScript config file using oxc-transform
 */
async function loadTypeScriptConfig(filePath: string): Promise<VizeConfig> {
  const source = fs.readFileSync(filePath, 'utf-8')
  const result = transform(filePath, source, {
    typescript: {
      onlyRemoveTypeImports: true,
    },
  })

  const code = result.code

  // Write to temp file and import
  const tempFile = filePath.replace(/\.m?ts$/, '.temp.mjs')
  fs.writeFileSync(tempFile, code)

  try {
    const fileUrl = pathToFileURL(tempFile).href
    const module = await import(fileUrl)
    return module.default || module
  } finally {
    fs.unlinkSync(tempFile)
  }
}

/**
 * Load ESM config file
 */
async function loadESMConfig(filePath: string): Promise<VizeConfig> {
  const fileUrl = pathToFileURL(filePath).href
  const module = await import(fileUrl)
  return module.default || module
}

/**
 * Load CommonJS config file
 */
async function loadCommonJSConfig(filePath: string): Promise<VizeConfig> {
  // Use dynamic import for CJS files
  const module = await import(pathToFileURL(filePath).href)
  return module.default || module
}
