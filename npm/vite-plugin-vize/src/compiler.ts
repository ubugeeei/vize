import fs from "node:fs";
import * as native from "@vizejs/native";
import type {
  CompiledModule,
  BatchFileInput,
  BatchCompileResultWithFiles,
  StyleBlockInfo,
} from "./types.js";
import { generateScopeId } from "./utils.js";

const { compileSfc, compileSfcBatchWithResults } = native;

/**
 * Extract style block metadata from a Vue SFC source string.
 * Parses `<style>` tags to determine lang, scoped, and module attributes.
 */
export function extractStyleBlocks(source: string): StyleBlockInfo[] {
  const blocks: StyleBlockInfo[] = [];
  const styleRegex = /<style([^>]*)>([\s\S]*?)<\/style>/gi;
  let match;
  let index = 0;
  while ((match = styleRegex.exec(source)) !== null) {
    const attrs = match[1];
    const content = match[2];
    const lang = attrs.match(/\blang=["']([^"']+)["']/)?.[1] ?? null;
    const scoped = /\bscoped\b/.test(attrs);
    const moduleMatch = attrs.match(/\bmodule(?:=["']([^"']+)["'])?\b/);
    const isModule = moduleMatch ? moduleMatch[1] || true : false;
    blocks.push({ content, lang, scoped, module: isModule, index });
    index++;
  }
  return blocks;
}

export function compileFile(
  filePath: string,
  cache: Map<string, CompiledModule>,
  options: { sourceMap: boolean; ssr: boolean },
  source?: string,
): CompiledModule {
  const content = source ?? fs.readFileSync(filePath, "utf-8");
  const scopeId = generateScopeId(filePath);
  const hasScoped = /<style[^>]*\bscoped\b/.test(content);

  const result = compileSfc(content, {
    filename: filePath,
    sourceMap: options.sourceMap,
    ssr: options.ssr,
    scopeId: hasScoped ? `data-v-${scopeId}` : undefined,
  });

  if (result.errors.length > 0) {
    const errorMsg = result.errors.join("\n");
    console.error(`[vize] Compilation error in ${filePath}:\n${errorMsg}`);
  }

  if (result.warnings.length > 0) {
    result.warnings.forEach((warning) => {
      console.warn(`[vize] Warning in ${filePath}: ${warning}`);
    });
  }

  const styles = extractStyleBlocks(content);

  const compiled: CompiledModule = {
    code: result.code,
    css: result.css,
    scopeId,
    hasScoped,
    styles,
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
  options: { ssr: boolean },
): BatchCompileResultWithFiles {
  const inputs: BatchFileInput[] = files.map((f) => ({
    path: f.path,
    source: f.source,
  }));

  const result = compileSfcBatchWithResults(inputs, {
    ssr: options.ssr,
  });

  // Build a map from path -> source for style block extraction
  const sourceMap = new Map<string, string>();
  for (const f of files) {
    sourceMap.set(f.path, f.source);
  }

  // Update cache with results
  for (const fileResult of result.results) {
    if (fileResult.errors.length === 0) {
      const source = sourceMap.get(fileResult.path);
      const styles = source ? extractStyleBlocks(source) : undefined;
      cache.set(fileResult.path, {
        code: fileResult.code,
        css: fileResult.css,
        scopeId: fileResult.scopeId,
        hasScoped: fileResult.hasScoped,
        templateHash: fileResult.templateHash,
        styleHash: fileResult.styleHash,
        scriptHash: fileResult.scriptHash,
        styles,
      });
    }

    // Log errors and warnings
    if (fileResult.errors.length > 0) {
      console.error(
        `[vize] Compilation error in ${fileResult.path}:\n${fileResult.errors.join("\n")}`,
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
