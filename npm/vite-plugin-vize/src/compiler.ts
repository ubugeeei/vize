import fs from 'node:fs';
import { createRequire } from 'node:module';
import type {
  CompileSfcFn,
  CompileSfcBatchWithResultsFn,
  CompiledModule,
  SfcCompileOptionsNapi,
  BatchFileInput,
  BatchCompileOptionsNapi,
  BatchCompileResultWithFiles,
} from './types.js';
import { generateScopeId } from './utils.js';

const require = createRequire(import.meta.url);

let compileSfc: CompileSfcFn | null = null;
let compileSfcBatchWithResults: CompileSfcBatchWithResultsFn | null = null;

export function loadNative(): CompileSfcFn {
  if (compileSfc) return compileSfc;

  try {
    const native = require('@vizejs/native');
    compileSfc = native.compileSfc;
    return compileSfc!;
  } catch (e) {
    throw new Error(
      `Failed to load @vizejs/native. Make sure it's installed and built:\n${e}`
    );
  }
}

export function loadNativeBatch(): CompileSfcBatchWithResultsFn {
  if (compileSfcBatchWithResults) return compileSfcBatchWithResults;

  try {
    const native = require('@vizejs/native');
    compileSfcBatchWithResults = native.compileSfcBatchWithResults;
    return compileSfcBatchWithResults!;
  } catch (e) {
    throw new Error(
      `Failed to load @vizejs/native. Make sure it's installed and built:\n${e}`
    );
  }
}

export function compileFile(
  filePath: string,
  cache: Map<string, CompiledModule>,
  options: { sourceMap: boolean; ssr: boolean },
  source?: string
): CompiledModule {
  const compile = loadNative();
  const content = source ?? fs.readFileSync(filePath, 'utf-8');
  const scopeId = generateScopeId(filePath);
  const hasScoped = /<style[^>]*\bscoped\b/.test(content);

  const result = compile(content, {
    filename: filePath,
    sourceMap: options.sourceMap,
    ssr: options.ssr,
    scopeId: hasScoped ? `data-v-${scopeId}` : undefined,
  });

  if (result.errors.length > 0) {
    const errorMsg = result.errors.join('\n');
    console.error(`[vize] Compilation error in ${filePath}:\n${errorMsg}`);
  }

  if (result.warnings.length > 0) {
    result.warnings.forEach((warning) => {
      console.warn(`[vize] Warning in ${filePath}: ${warning}`);
    });
  }

  const compiled: CompiledModule = {
    code: result.code,
    css: result.css,
    scopeId,
    hasScoped,
  };

  cache.set(filePath, compiled);
  return compiled;
}

/**
 * Batch compile multiple files in parallel using native Rust multithreading.
 * Returns per-file results with content hashes for HMR.
 */
export function compileBatch(
  files: { path: string; source: string }[],
  cache: Map<string, CompiledModule>,
  options: { ssr: boolean }
): BatchCompileResultWithFiles {
  const compile = loadNativeBatch();

  const inputs: BatchFileInput[] = files.map((f) => ({
    path: f.path,
    source: f.source,
  }));

  const result = compile(inputs, {
    ssr: options.ssr,
  });

  // Update cache with results
  for (const fileResult of result.results) {
    if (fileResult.errors.length === 0) {
      cache.set(fileResult.path, {
        code: fileResult.code,
        css: fileResult.css,
        scopeId: fileResult.scopeId,
        hasScoped: fileResult.hasScoped,
        templateHash: fileResult.templateHash,
        styleHash: fileResult.styleHash,
        scriptHash: fileResult.scriptHash,
      });
    }

    // Log errors and warnings
    if (fileResult.errors.length > 0) {
      console.error(
        `[vize] Compilation error in ${fileResult.path}:\n${fileResult.errors.join('\n')}`
      );
    }
    if (fileResult.warnings.length > 0) {
      fileResult.warnings.forEach((warning) => {
        console.warn(`[vize] Warning in ${fileResult.path}: ${warning}`);
      });
    }
  }

  return result;
}
