/**
 * Lint Benchmark: Vize (patina) vs eslint-plugin-vue
 *
 * Usage:
 *   1. Generate test files: node generate.mjs [count]
 *   2. Build CLI: mise run build:cli
 *   3. Run benchmark: node --experimental-strip-types bench/lint.ts
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
const VIZE_BIN =
  join(__dirname, "..", "target", "release", "vize");
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

const totalSize = vueFiles.reduce(
  (sum, f) =>
    sum + readFileSync(join(INPUT_DIR, f), "utf-8").length,
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

// ESLint single-thread: use Node.js API
async function runEslintSingleThread(): Promise<number> {
  const { ESLint } = await import("eslint");
  const eslint = new ESLint({
    overrideConfigFile: join(INPUT_DIR, "eslint.config.mjs"),
    cwd: INPUT_DIR,
  });

  // Warmup
  const warmupFiles = vueFiles.slice(0, 5).map((f) => join(INPUT_DIR, f));
  await eslint.lintFiles(warmupFiles);

  const start = performance.now();
  const filePaths = vueFiles.map((f) => join(INPUT_DIR, f));
  await eslint.lintFiles(filePaths);
  return performance.now() - start;
}

// ESLint multi-thread: worker threads
async function runEslintMultiThread(): Promise<number> {
  const workerCount = CPU_COUNT;
  const chunkSize = Math.ceil(vueFiles.length / workerCount);

  const workerCode = `
    const { parentPort, workerData } = require('worker_threads');
    const { ESLint } = require('eslint');

    (async () => {
      const eslint = new ESLint({
        overrideConfigFile: workerData.configFile,
        cwd: workerData.cwd,
      });
      await eslint.lintFiles(workerData.files);
      parentPort.postMessage('done');
    })();
  `;

  const start = performance.now();

  const workers: Promise<unknown>[] = [];
  for (let i = 0; i < workerCount; i++) {
    const startIdx = i * chunkSize;
    const endIdx = Math.min(startIdx + chunkSize, vueFiles.length);
    const chunk = vueFiles.slice(startIdx, endIdx).map((f) => join(INPUT_DIR, f));

    const worker = new Worker(workerCode, {
      eval: true,
      workerData: {
        files: chunk,
        configFile: join(INPUT_DIR, "eslint.config.mjs"),
        cwd: INPUT_DIR,
      },
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

// Run shell command, ignoring exit code (linter may exit non-zero on warnings)
function execIgnoreExit(cmd: string): void {
  try {
    execSync(cmd, { stdio: "ignore" });
  } catch {
    // ignore non-zero exit code
  }
}

// Vize (patina) single-thread
function runVizeLintSingleThread(): number {
  // Warmup
  for (let i = 0; i < 3; i++) {
    execIgnoreExit(
      `RAYON_NUM_THREADS=1 ${VIZE_BIN} lint '${GLOB_PATTERN}'`
    );
  }

  const start = performance.now();
  execIgnoreExit(
    `RAYON_NUM_THREADS=1 ${VIZE_BIN} lint '${GLOB_PATTERN}'`
  );
  return performance.now() - start;
}

// Vize (patina) multi-thread
function runVizeLintMultiThread(): number {
  // Warmup
  for (let i = 0; i < 3; i++) {
    execIgnoreExit(`${VIZE_BIN} lint '${GLOB_PATTERN}'`);
  }

  const start = performance.now();
  execIgnoreExit(`${VIZE_BIN} lint '${GLOB_PATTERN}'`);
  return performance.now() - start;
}

// Main
console.log();
console.log("=".repeat(65));
console.log(" Lint Benchmark: patina vs eslint-plugin-vue");
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

const eslintSingle = await runEslintSingleThread();
console.log(
  `   eslint-plugin-vue : ${formatTime(eslintSingle).padStart(8)}  (${formatThroughput(vueFiles.length, eslintSingle)})`
);

let vizeSingle = 0;
if (existsSync(VIZE_BIN)) {
  vizeSingle = runVizeLintSingleThread();
  const speedup = (eslintSingle / vizeSingle).toFixed(1);
  console.log(
    `   Vize (patina)     : ${formatTime(vizeSingle).padStart(8)}  (${formatThroughput(vueFiles.length, vizeSingle)})  ${speedup}x faster`
  );
} else {
  console.log("   Vize (patina)     : SKIPPED (vize CLI not found)");
}

// Multi Thread
console.log();
console.log(` Multi Thread (${CPU_COUNT} workers):`);
console.log();

const eslintMulti = await runEslintMultiThread();
console.log(
  `   eslint-plugin-vue : ${formatTime(eslintMulti).padStart(8)}  (${formatThroughput(vueFiles.length, eslintMulti)})`
);

let vizeMulti = 0;
if (existsSync(VIZE_BIN)) {
  vizeMulti = runVizeLintMultiThread();
  const speedup = (eslintMulti / vizeMulti).toFixed(1);
  console.log(
    `   Vize (patina)     : ${formatTime(vizeMulti).padStart(8)}  (${formatThroughput(vueFiles.length, vizeMulti)})  ${speedup}x faster`
  );
} else {
  console.log("   Vize (patina)     : SKIPPED (vize CLI not found)");
}

// Summary
if (vizeSingle > 0 && vizeMulti > 0) {
  console.log();
  console.log("-".repeat(65));
  console.log();
  console.log(" Summary:");
  console.log();
  const stSpeedup = (eslintSingle / vizeSingle).toFixed(1);
  const mtSpeedup = (eslintMulti / vizeMulti).toFixed(1);
  const crossSpeedup = (eslintSingle / vizeMulti).toFixed(1);
  console.log(`   eslint ST vs Vize ST : ${stSpeedup}x`);
  console.log(`   eslint MT vs Vize MT : ${mtSpeedup}x`);
  console.log(`   eslint ST vs Vize MT : ${crossSpeedup}x  (user-facing speedup)`);
}

console.log();
