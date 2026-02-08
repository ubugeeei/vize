#!/usr/bin/env node --experimental-strip-types
/**
 * Generate expected outputs from Vue's official compiler.
 *
 * Usage:
 *   node --experimental-strip-types scripts/generate-expected.ts              # All modes
 *   node --experimental-strip-types scripts/generate-expected.ts sfc/basic    # Specific fixture
 *   node --experimental-strip-types scripts/generate-expected.ts --mode vdom  # All vdom fixtures
 *   node --experimental-strip-types scripts/generate-expected.ts --mode vapor # All vapor fixtures
 *   node --experimental-strip-types scripts/generate-expected.ts --mode sfc   # All sfc fixtures
 */

import * as fs from 'node:fs'
import * as path from 'node:path'
import { parse as parseTOML } from '@iarna/toml'
import { compileScript, compileTemplate, parse as parseSFC } from '@vue/compiler-sfc'

interface TestCase {
  name: string
  input: string
  options?: Record<string, any>
}

interface TestFixture {
  mode?: string
  cases: TestCase[]
}

const fixturesDir = path.join(import.meta.dirname!, '..', 'tests', 'fixtures')
const expectedDir = path.join(import.meta.dirname!, '..', 'tests', 'expected')

function loadFixture(filePath: string): TestFixture {
  const content = fs.readFileSync(filePath, 'utf-8')
  return parseTOML(content) as any
}

function compileVdom(source: string, options?: Record<string, any>): string {
  const result = compileTemplate({
    source,
    filename: 'test.vue',
    id: 'test',
    compilerOptions: {
      mode: 'module',
      prefixIdentifiers: true,
      hoistStatic: options?.hoistStatic ?? false,
      cacheHandlers: options?.cacheHandlers ?? false,
      ssr: options?.ssr ?? false,
    },
  })
  if (result.errors.length > 0) {
    return `Compile errors: ${result.errors.map(e => typeof e === 'string' ? e : e.message).join(', ')}`
  }
  return result.code
}

let _compileVaporFn: any = null
async function loadVaporCompiler() {
  if (!_compileVaporFn) {
    const mod = await import('@vue/compiler-vapor')
    _compileVaporFn = mod.compile
  }
  return _compileVaporFn
}

function compileVapor(source: string, _options?: Record<string, any>): string {
  if (!_compileVaporFn) {
    return `// @vue/compiler-vapor not loaded`
  }
  try {
    const result = _compileVaporFn(source, {
      mode: 'module',
      prefixIdentifiers: false,
    })
    return result.code
  } catch (e) {
    return `// Compile error: ${e}`
  }
}

function compileSFC(source: string, filename: string = 'test.vue', isTS: boolean = false): string {
  const { descriptor, errors } = parseSFC(source, { filename })

  if (errors.length > 0) {
    return `Parse errors: ${errors.map(e => e.message).join(', ')}`
  }

  // Detect if script is TypeScript
  const scriptLang = descriptor.scriptSetup?.lang || descriptor.script?.lang
  const actualIsTS = isTS || scriptLang === 'ts' || scriptLang === 'tsx'

  let code = ''

  // Compile template if present
  let templateResult: ReturnType<typeof compileTemplate> | null = null
  if (descriptor.template) {
    const bindings = descriptor.scriptSetup ? compileScript(descriptor, {
      id: filename,
      templateOptions: {}
    }).bindings : undefined

    templateResult = compileTemplate({
      source: descriptor.template.content,
      filename,
      id: filename,
      compilerOptions: {
        bindingMetadata: bindings,
        mode: 'module',
      },
    })
  }

  // Compile script setup
  if (descriptor.scriptSetup) {
    const result = compileScript(descriptor, {
      id: filename,
      inlineTemplate: true,
      templateOptions: templateResult ? {
        compilerOptions: {
          mode: 'module',
        }
      } : undefined,
    })
    code = result.content
  } else if (descriptor.script) {
    // Options API - rewrite export default
    const scriptContent = descriptor.script.content
    const rewritten = scriptContent.replace(
      /export\s+default\s+\{/,
      'const _sfc_main = {'
    )

    if (templateResult) {
      code = templateResult.code + '\n' + rewritten + '\n_sfc_main.render = _sfc_render\nexport default _sfc_main'
    } else {
      code = rewritten + '\nexport default _sfc_main'
    }
  } else if (templateResult) {
    // Template only
    code = templateResult.code
  }

  return code
}

function formatSnapFile(cases: Array<{name: string, input: string, output: string, mode: string, css?: string}>): string {
  let content = ''

  for (const testCase of cases) {
    content += `===\n`
    content += `name: ${testCase.name}\n`
    content += `options: ${testCase.mode}\n`
    content += `--- INPUT ---\n`
    content += testCase.input.trim() + '\n'
    content += `--- OUTPUT ---\n`
    content += testCase.output.trim() + '\n'
    if (testCase.css) {
      content += `--- CSS ---\n`
      content += testCase.css.trim() + '\n'
    }
  }

  return content
}

function processFixture(fixturePath: string, outputPath: string) {
  console.log(`Processing: ${path.relative(fixturesDir, fixturePath)}`)

  const fixture = loadFixture(fixturePath)
  const mode = fixture.mode || 'vdom'
  const results: Array<{name: string, input: string, output: string, mode: string}> = []

  for (const testCase of fixture.cases) {
    try {
      let output: string
      switch (mode) {
        case 'vdom':
          output = compileVdom(testCase.input, testCase.options)
          break
        case 'vapor':
          output = compileVapor(testCase.input, testCase.options)
          break
        case 'sfc':
          output = compileSFC(testCase.input, 'test.vue')
          break
        default:
          output = `Unknown mode: ${mode}`
      }
      results.push({
        name: testCase.name,
        input: testCase.input,
        output,
        mode,
      })
    } catch (error) {
      console.error(`  Error compiling "${testCase.name}":`, error)
      results.push({
        name: testCase.name,
        input: testCase.input,
        output: `Compile error: ${error}`,
        mode,
      })
    }
  }

  // Ensure output directory exists
  fs.mkdirSync(path.dirname(outputPath), { recursive: true })

  // Write snap file
  const content = formatSnapFile(results)
  fs.writeFileSync(outputPath, content)
  console.log(`  Written: ${path.relative(expectedDir, outputPath)} (${results.length} cases)`)
}

function processDir(dirName: string) {
  const dir = path.join(fixturesDir, dirName)
  if (!fs.existsSync(dir)) return
  const files = fs.readdirSync(dir).filter(f => f.endsWith('.toml'))
  for (const file of files) {
    const fixturePath = path.join(dir, file)
    const outputPath = path.join(expectedDir, dirName, file.replace('.toml', '.snap'))
    processFixture(fixturePath, outputPath)
  }
}

async function main() {
  // Pre-load vapor compiler
  await loadVaporCompiler().catch(() => {
    console.warn('Warning: @vue/compiler-vapor not available, vapor fixtures will be skipped')
  })
  const args = process.argv.slice(2)

  // Handle --mode flag
  const modeIdx = args.indexOf('--mode')
  if (modeIdx !== -1 && args[modeIdx + 1]) {
    const mode = args[modeIdx + 1]
    processDir(mode)
    return
  }

  // If specific fixture is provided
  if (args.length > 0 && !args[0].startsWith('--')) {
    for (const arg of args) {
      const fixturePath = path.join(fixturesDir, `${arg}.toml`)
      const outputPath = path.join(expectedDir, `${arg}.snap`)

      if (!fs.existsSync(fixturePath)) {
        console.error(`Fixture not found: ${fixturePath}`)
        continue
      }

      processFixture(fixturePath, outputPath)
    }
    return
  }

  // Process all modes
  for (const mode of ['vdom', 'vapor', 'sfc']) {
    processDir(mode)
  }
}

main()
