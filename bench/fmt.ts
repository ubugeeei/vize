/**
 * Format Benchmark: Vize (glyph) vs Prettier
 *
 * Usage:
 *   1. Generate test files: node generate.mjs [count]
 *   2. Build CLI: mise run build:cli
 *   3. Run benchmark: node --experimental-strip-types bench/fmt.ts
 */

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { execSync } from "node:child_process";
import { Worker } from "node:worker_threads";
import os from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const INPUT_DIR = join(__dirname, "__in__");
const CPU_COUNT = os.cpus().length;
const VIZE_BIN = join(__dirname, "..", "target", "release", "vize");
const GLOB_PATTERN = join(INPUT_DIR, "*.vue");

// Check input files
if (!existsSync(INPUT_DIR)) {
  console.error(
    `Error: Input directory not found: ${INPUT_DIR}\nRun 'node generate.mjs' first.`
  );
  process.exit(1);
}

const vueFiles = readdirSync(INPUT_DIR).filter((f) => f.endsWith(".vue"));
if (vueFiles.length === 0) {
  console.error(
    `Error: No .vue files found in ${INPUT_DIR}\nRun 'node generate.mjs' first.`
  );
  process.exit(1);
}

interface FileData {
  filename: string;
  source: string;
}

const files: FileData[] = vueFiles.map((filename) => ({
  filename,
  source: readFileSync(join(INPUT_DIR, filename), "utf-8"),
}));

const totalSize = files.reduce(
  (sum, f) => sum + Buffer.byteLength(f.source, "utf8"),
  0
);

// Format helpers
function formatTime(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
  return `${ms.toFixed(0)}ms`;
}

function formatThroughput(fileCount: number, ms: number): string {
  const filesPerSec = (fileCount / ms) * 1000;
  if (filesPerSec >= 1000) return `${(filesPerSec / 1000).toFixed(1)}k files/s`;
  return `${filesPerSec.toFixed(0)} files/s`;
}

function formatBytesPerSec(bytes: number, ms: number): string {
  const bps = (bytes / ms) * 1000;
  if (bps >= 1024 * 1024) return `${(bps / 1024 / 1024).toFixed(1)} MB/s`;
  if (bps >= 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
  return `${bps.toFixed(0)} B/s`;
}

// Run shell command, ignoring exit code (formatter may exit non-zero on diffs)
function execIgnoreExit(cmd: string): void {
  try {
    execSync(cmd, { stdio: "ignore" });
  } catch {
    // ignore non-zero exit code
  }
}

// Prettier single-thread
async function runPrettierSingleThread(): Promise<number> {
  const prettier = await import("prettier");

  // Warmup
  for (let i = 0; i < 3; i++) {
    for (const file of files.slice(0, 10)) {
      await prettier.format(file.source, { parser: "vue" });
    }
  }

  const start = performance.now();
  for (const file of files) {
    await prettier.format(file.source, { parser: "vue" });
  }
  return performance.now() - start;
}

// Prettier multi-thread (worker threads)
async function runPrettierMultiThread(): Promise<number> {
  const workerCount = CPU_COUNT;
  const chunkSize = Math.ceil(files.length / workerCount);

  const workerCode = `
    const { parentPort, workerData } = require('worker_threads');
    const prettier = require('prettier');

    (async () => {
      for (const file of workerData.files) {
        await prettier.format(file.source, { parser: 'vue' });
      }
      parentPort.postMessage('done');
    })();
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

// Vize (glyph) single-thread
function runVizeFmtSingleThread(): number {
  // Warmup
  for (let i = 0; i < 3; i++) {
    execIgnoreExit(
      `RAYON_NUM_THREADS=1 ${VIZE_BIN} fmt --check '${GLOB_PATTERN}'`
    );
  }

  const start = performance.now();
  execIgnoreExit(
    `RAYON_NUM_THREADS=1 ${VIZE_BIN} fmt --check '${GLOB_PATTERN}'`
  );
  return performance.now() - start;
}

// Vize (glyph) multi-thread
function runVizeFmtMultiThread(): number {
  // Warmup
  for (let i = 0; i < 3; i++) {
    execIgnoreExit(`${VIZE_BIN} fmt --check '${GLOB_PATTERN}'`);
  }

  const start = performance.now();
  execIgnoreExit(`${VIZE_BIN} fmt --check '${GLOB_PATTERN}'`);
  return performance.now() - start;
}

// Main
console.log();
console.log("=".repeat(65));
console.log(" Format Benchmark: glyph vs prettier");
console.log("=".repeat(65));
console.log();
console.log(` Files     : ${vueFiles.length.toLocaleString()} SFC files`);
console.log(` Total Size: ${(totalSize / 1024 / 1024).toFixed(1)} MB`);
console.log(` CPU Cores : ${CPU_COUNT}`);
console.log();
console.log("-".repeat(65));

// Single Thread
console.log();
console.log(" Single Thread:");
console.log();

const prettierSingle = await runPrettierSingleThread();
console.log(
  `   Prettier      : ${formatTime(prettierSingle).padStart(8)}  (${formatThroughput(vueFiles.length, prettierSingle)}, ${formatBytesPerSec(totalSize, prettierSingle)})`
);

let vizeSingle = 0;
if (existsSync(VIZE_BIN)) {
  vizeSingle = runVizeFmtSingleThread();
  const speedup = (prettierSingle / vizeSingle).toFixed(1);
  console.log(
    `   Vize (glyph)  : ${formatTime(vizeSingle).padStart(8)}  (${formatThroughput(vueFiles.length, vizeSingle)}, ${formatBytesPerSec(totalSize, vizeSingle)})  ${speedup}x faster`
  );
} else {
  console.log("   Vize (glyph)  : SKIPPED (vize CLI not found)");
}

// Multi Thread
console.log();
console.log(` Multi Thread (${CPU_COUNT} workers):`);
console.log();

const prettierMulti = await runPrettierMultiThread();
console.log(
  `   Prettier      : ${formatTime(prettierMulti).padStart(8)}  (${formatThroughput(vueFiles.length, prettierMulti)}, ${formatBytesPerSec(totalSize, prettierMulti)})`
);

let vizeMulti = 0;
if (existsSync(VIZE_BIN)) {
  vizeMulti = runVizeFmtMultiThread();
  const speedup = (prettierMulti / vizeMulti).toFixed(1);
  console.log(
    `   Vize (glyph)  : ${formatTime(vizeMulti).padStart(8)}  (${formatThroughput(vueFiles.length, vizeMulti)}, ${formatBytesPerSec(totalSize, vizeMulti)})  ${speedup}x faster`
  );
} else {
  console.log("   Vize (glyph)  : SKIPPED (vize CLI not found)");
}

// Summary
if (vizeSingle > 0 && vizeMulti > 0) {
  console.log();
  console.log("-".repeat(65));
  console.log();
  console.log(" Summary (vs Prettier):");
  console.log();
  const stSpeedup = (prettierSingle / vizeSingle).toFixed(1);
  const mtSpeedup = (prettierMulti / vizeMulti).toFixed(1);
  const crossSpeedup = (prettierSingle / vizeMulti).toFixed(1);
  console.log(`   Prettier ST vs Vize ST : ${stSpeedup}x`);
  console.log(`   Prettier MT vs Vize MT : ${mtSpeedup}x`);
  console.log(
    `   Prettier ST vs Vize MT : ${crossSpeedup}x  (user-facing speedup)`
  );
}

console.log();
