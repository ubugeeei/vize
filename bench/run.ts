/**
 * SFC Compile Benchmark
 *
 * Compares @vue/compiler-sfc with vize native batch compilation.
 *
 * Usage:
 *   1. Generate test files: node generate.mjs [count]
 *   2. Build native bindings: mise run build
 *   3. Run benchmark: node --experimental-strip-types run.ts
 */

import { parse, compileScript, compileTemplate } from "@vue/compiler-sfc";
import type { BindingMetadata } from "@vue/compiler-sfc";
import {
  existsSync,
  readdirSync,
  readFileSync,
  writeFileSync,
  mkdirSync,
} from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join, basename } from "node:path";
import { createRequire } from "node:module";
import { Worker } from "node:worker_threads";
import os from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

const INPUT_DIR = join(__dirname, "__in__");
const OUTPUT_DIR = join(__dirname, "__out__");
const ORIGINAL_OUT_DIR = join(OUTPUT_DIR, "original");
const NATIVE_OUT_DIR = join(OUTPUT_DIR, "native");
const CPU_COUNT = os.cpus().length;
const FILE_LIMIT = parseInt(process.argv[2] || "0", 10) || Infinity;

// Types
interface NativeBindings {
  compileSfc: (
    source: string,
    options: { filename: string }
  ) => { code: string };
  compileSfcBatch: (
    pattern: string,
    options?: { ssr?: boolean; threads?: number }
  ) => {
    success: number;
    failed: number;
    inputBytes: number;
    outputBytes: number;
    timeMs: number;
  };
}

interface TestFile {
  filename: string;
  source: string;
}

interface CompileResult {
  filename: string;
  code: string;
}

// Load Native (NAPI) bindings
let native: NativeBindings | null = null;
const nativePath = join(__dirname, "..", "npm", "vize-native");
if (existsSync(nativePath)) {
  try {
    native = require(nativePath) as NativeBindings;
  } catch (e) {
    console.error("Native load error:", (e as Error).message);
  }
}

// Check input files exist
if (!existsSync(INPUT_DIR)) {
  console.error(
    `Error: Input directory not found: ${INPUT_DIR}\nRun 'node generate.mjs' first to create test files.`
  );
  process.exit(1);
}

const vueFiles = readdirSync(INPUT_DIR)
  .filter((f) => f.endsWith(".vue"))
  .slice(0, FILE_LIMIT);
if (vueFiles.length === 0) {
  console.error(
    `Error: No .vue files found in ${INPUT_DIR}\nRun 'node generate.mjs' first to create test files.`
  );
  process.exit(1);
}

// Load test files
const files: TestFile[] = vueFiles.map((filename) => ({
  filename,
  source: readFileSync(join(INPUT_DIR, filename), "utf-8"),
}));

const totalSize = files.reduce(
  (sum, f) => sum + Buffer.byteLength(f.source, "utf8"),
  0
);

// Ensure output directories exist
mkdirSync(ORIGINAL_OUT_DIR, { recursive: true });
mkdirSync(NATIVE_OUT_DIR, { recursive: true });

// Vue compiler-sfc full compile
function vueCompileSfc(source: string, filename: string): string {
  const { descriptor } = parse(source, { filename });
  let bindings: BindingMetadata = {};
  let scriptCode = "";

  if (descriptor.scriptSetup || descriptor.script) {
    const scriptResult = compileScript(descriptor, { id: filename });
    bindings = scriptResult.bindings || {};
    scriptCode = scriptResult.content;
  }

  let templateCode = "";
  if (descriptor.template) {
    const templateResult = compileTemplate({
      source: descriptor.template.content,
      filename,
      id: filename,
      compilerOptions: { bindingMetadata: bindings },
    });
    templateCode = templateResult.code;
  }

  return `// ${filename}\n${scriptCode}\n${templateCode}`;
}

// Benchmark function (single-threaded)
function benchmark(fn: () => void): number {
  // Warmup
  for (let i = 0; i < 3; i++) fn();

  const start = performance.now();
  fn();
  return performance.now() - start;
}

// Single-threaded benchmark for original (with output)
function runOriginalSingleThread(saveOutput: boolean): number {
  return benchmark(() => {
    for (const file of files) {
      const code = vueCompileSfc(file.source, file.filename);
      if (saveOutput) {
        const outPath = join(
          ORIGINAL_OUT_DIR,
          file.filename.replace(".vue", ".js")
        );
        writeFileSync(outPath, code);
      }
    }
  });
}

// Single-threaded benchmark for native (with output)
function runNativeSingleThread(saveOutput: boolean): number {
  if (!native) return 0;
  return benchmark(() => {
    for (const file of files) {
      const result = native!.compileSfc(file.source, {
        filename: file.filename,
      });
      if (saveOutput) {
        const outPath = join(
          NATIVE_OUT_DIR,
          file.filename.replace(".vue", ".js")
        );
        writeFileSync(outPath, result.code);
      }
    }
  });
}

// Multi-threaded benchmark using Worker threads for original
async function runOriginalMultiThread(): Promise<number> {
  const workerCount = CPU_COUNT;
  const chunkSize = Math.ceil(files.length / workerCount);

  const workerCode = `
    const { parentPort, workerData } = require('worker_threads');
    const { parse, compileScript, compileTemplate } = require('@vue/compiler-sfc');

    for (const file of workerData.files) {
      const { descriptor } = parse(file.source, { filename: file.filename });
      let bindings = {};
      if (descriptor.scriptSetup || descriptor.script) {
        const scriptResult = compileScript(descriptor, { id: file.filename });
        bindings = scriptResult.bindings || {};
      }
      if (descriptor.template) {
        compileTemplate({
          source: descriptor.template.content,
          filename: file.filename,
          id: file.filename,
          compilerOptions: { bindingMetadata: bindings },
        });
      }
    }
    parentPort.postMessage('done');
  `;

  const start = performance.now();

  const workers: Promise<unknown>[] = [];
  for (let i = 0; i < workerCount; i++) {
    const startIdx = i * chunkSize;
    const endIdx = Math.min(startIdx + chunkSize, files.length);
    const chunk = files.slice(startIdx, endIdx);

    const worker = new Worker(workerCode, {
      eval: true,
      workerData: { files: chunk },
    });

    workers.push(
      new Promise((resolve, reject) => {
        worker.on("message", resolve);
        worker.on("error", reject);
      })
    );
  }

  await Promise.all(workers);
  return performance.now() - start;
}

// Format time
function formatTime(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
  return `${ms.toFixed(0)}ms`;
}

// Format throughput
function formatThroughput(fileCount: number, ms: number): string {
  const filesPerSec = (fileCount / ms) * 1000;
  if (filesPerSec >= 1000) return `${(filesPerSec / 1000).toFixed(1)}k files/s`;
  return `${filesPerSec.toFixed(0)} files/s`;
}

// Main
console.log();
console.log("=".repeat(65));
console.log(" SFC -> JS Compile Benchmark");
console.log("=".repeat(65));
console.log();
console.log(` Files     : ${files.length.toLocaleString()} SFC files`);
console.log(` Total Size: ${(totalSize / 1024 / 1024).toFixed(1)} MB`);
console.log(` CPU Cores : ${CPU_COUNT}`);
console.log(` Output    : ${OUTPUT_DIR}`);
console.log();
console.log(" Compilers:");
console.log(`   Original : @vue/compiler-sfc`);
console.log(
  `   Native   : vize (NAPI)  ${native ? "OK" : "NOT FOUND"}`
);
console.log();
console.log("-".repeat(65));

// Single-threaded benchmarks
console.log();
console.log(" Single Thread:");
console.log();

// First run saves output (skip in quick mode)
const saveOutput = FILE_LIMIT === Infinity;
const originalSingle = runOriginalSingleThread(saveOutput);
console.log(
  `   Original : ${formatTime(originalSingle).padStart(8)}  (${formatThroughput(files.length, originalSingle)})`
);

if (native) {
  const nativeSingle = runNativeSingleThread(saveOutput);
  const speedup = (originalSingle / nativeSingle).toFixed(1);
  console.log(
    `   Native   : ${formatTime(nativeSingle).padStart(8)}  (${formatThroughput(files.length, nativeSingle)})  ${speedup}x faster`
  );
}

// Multi-threaded benchmarks
console.log();
if (FILE_LIMIT === Infinity) {
  console.log(` Multi Thread (${CPU_COUNT} workers):`);
  console.log();

  const originalMulti = await runOriginalMultiThread();
  console.log(
    `   Original : ${formatTime(originalMulti).padStart(8)}  (${formatThroughput(files.length, originalMulti)})`
  );

  if (native) {
    // Use native batch compile with glob pattern (native multithreading via rayon)
    const pattern = join(INPUT_DIR, "*.vue");

    // Warmup
    for (let i = 0; i < 3; i++) {
      native.compileSfcBatch(pattern);
    }

    const result = native.compileSfcBatch(pattern);
    const nativeMulti = result.timeMs;
    const speedup = (originalMulti / nativeMulti).toFixed(1);
    console.log(
      `   Native   : ${formatTime(nativeMulti).padStart(8)}  (${formatThroughput(files.length, nativeMulti)})  ${speedup}x faster`
    );
  }
} else {
  // Quick mode: compare Original single-thread vs Native multi-thread (batch)
  console.log(` Quick Comparison (Original 1T vs Native MT):`);
  console.log();

  const originalThroughput = (files.length / originalSingle) * 1000; // files/sec

  if (native) {
    const pattern = join(INPUT_DIR, "*.vue");

    // Warmup
    for (let i = 0; i < 3; i++) {
      native.compileSfcBatch(pattern);
    }

    const result = native.compileSfcBatch(pattern);
    const nativeThroughput = (result.success / result.timeMs) * 1000; // files/sec
    const speedup = (nativeThroughput / originalThroughput).toFixed(1);

    console.log(
      `   Original : ${formatThroughput(files.length, originalSingle).padStart(12)}  (single-thread, ${files.length} files)`
    );
    console.log(
      `   Native   : ${formatThroughput(result.success, result.timeMs).padStart(12)}  (multi-thread, ${result.success} files)  ${speedup}x faster`
    );
  }
}

// Cross-mode summary: Original ST vs Native MT
if (FILE_LIMIT === Infinity && native) {
  const pattern = join(INPUT_DIR, "*.vue");
  const batchResult = native.compileSfcBatch(pattern);
  const nativeMultiMs = batchResult.timeMs;
  const crossSpeedup = (originalSingle / nativeMultiMs).toFixed(1);

  console.log();
  console.log("-".repeat(65));
  console.log();
  console.log(" Summary:");
  console.log();
  console.log(
    `   Original ST vs Native ST : ${(originalSingle / runNativeSingleThread(false)).toFixed(1)}x`
  );
  console.log(
    `   Original ST vs Native MT : ${crossSpeedup}x  (user-facing speedup)`
  );
}

console.log();
console.log("-".repeat(65));
if (FILE_LIMIT === Infinity) {
  console.log();
  console.log(` Output saved to:`);
  console.log(`   Original : ${ORIGINAL_OUT_DIR}`);
  console.log(`   Native   : ${NATIVE_OUT_DIR}`);
}
console.log();
