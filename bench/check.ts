/**
 * Type Check Benchmark: Vize (canon) vs vue-tsc
 *
 * Usage:
 *   1. Generate test files: node generate.mjs [count]
 *   2. Build CLI: mise run build:cli
 *   3. Run benchmark: node --experimental-strip-types bench/check.ts
 */

import { existsSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { execSync } from "node:child_process";
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

if (!existsSync(join(INPUT_DIR, "tsconfig.json"))) {
  console.error(
    `Error: tsconfig.json not found in ${INPUT_DIR}\nRun 'node generate.mjs' first to generate it.`
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

function runCommand(cmd: string): number {
  const start = performance.now();
  try {
    execSync(cmd, { stdio: "ignore", cwd: INPUT_DIR });
  } catch {
    // vue-tsc may exit non-zero on type errors; still measure time
  }
  return performance.now() - start;
}

function benchmarkCommand(cmd: string, warmup: number = 2): number {
  // Warmup
  for (let i = 0; i < warmup; i++) {
    runCommand(cmd);
  }
  return runCommand(cmd);
}

// vue-tsc single-thread
function runVueTscSingleThread(): number {
  const vueTscBin = join(__dirname, "..", "node_modules", ".bin", "vue-tsc");
  if (!existsSync(vueTscBin)) return -1;
  return benchmarkCommand(`${vueTscBin} --noEmit -p ${join(INPUT_DIR, "tsconfig.json")}`);
}

// vue-tsc multi-thread (default TS internal parallelism)
function runVueTscMultiThread(): number {
  const vueTscBin = join(__dirname, "..", "node_modules", ".bin", "vue-tsc");
  if (!existsSync(vueTscBin)) return -1;
  return benchmarkCommand(`${vueTscBin} --noEmit -p ${join(INPUT_DIR, "tsconfig.json")}`);
}

// Vize (canon) single-thread
function runVizeCheckSingleThread(): number {
  return benchmarkCommand(
    `RAYON_NUM_THREADS=1 ${VIZE_BIN} check '${GLOB_PATTERN}'`
  );
}

// Vize (canon) multi-thread
function runVizeCheckMultiThread(): number {
  return benchmarkCommand(`${VIZE_BIN} check '${GLOB_PATTERN}'`);
}

// Main
console.log();
console.log("=".repeat(65));
console.log(" Type Check Benchmark: canon vs vue-tsc");
console.log("=".repeat(65));
console.log();
console.log(` Files     : ${vueFiles.length.toLocaleString()} SFC files`);
console.log(` CPU Cores : ${CPU_COUNT}`);
console.log();
console.log("-".repeat(65));

// Single Thread
console.log();
console.log(" Single Thread:");
console.log();

const vueTscSingle = runVueTscSingleThread();
if (vueTscSingle >= 0) {
  console.log(
    `   vue-tsc       : ${formatTime(vueTscSingle).padStart(8)}  (${formatThroughput(vueFiles.length, vueTscSingle)})`
  );
} else {
  console.log("   vue-tsc       : SKIPPED (not found)");
}

let vizeSingle = 0;
if (existsSync(VIZE_BIN)) {
  vizeSingle = runVizeCheckSingleThread();
  if (vueTscSingle >= 0) {
    const speedup = (vueTscSingle / vizeSingle).toFixed(1);
    console.log(
      `   Vize (canon)  : ${formatTime(vizeSingle).padStart(8)}  (${formatThroughput(vueFiles.length, vizeSingle)})  ${speedup}x faster`
    );
  } else {
    console.log(
      `   Vize (canon)  : ${formatTime(vizeSingle).padStart(8)}  (${formatThroughput(vueFiles.length, vizeSingle)})`
    );
  }
} else {
  console.log("   Vize (canon)  : SKIPPED (vize CLI not found)");
}

// Multi Thread
console.log();
console.log(` Multi Thread:`);
console.log();

const vueTscMulti = runVueTscMultiThread();
if (vueTscMulti >= 0) {
  console.log(
    `   vue-tsc       : ${formatTime(vueTscMulti).padStart(8)}  (${formatThroughput(vueFiles.length, vueTscMulti)})`
  );
} else {
  console.log("   vue-tsc       : SKIPPED (not found)");
}

let vizeMulti = 0;
if (existsSync(VIZE_BIN)) {
  vizeMulti = runVizeCheckMultiThread();
  if (vueTscMulti >= 0) {
    const speedup = (vueTscMulti / vizeMulti).toFixed(1);
    console.log(
      `   Vize (canon)  : ${formatTime(vizeMulti).padStart(8)}  (${formatThroughput(vueFiles.length, vizeMulti)})  ${speedup}x faster`
    );
  } else {
    console.log(
      `   Vize (canon)  : ${formatTime(vizeMulti).padStart(8)}  (${formatThroughput(vueFiles.length, vizeMulti)})`
    );
  }
} else {
  console.log("   Vize (canon)  : SKIPPED (vize CLI not found)");
}

// Summary
if (vueTscSingle >= 0 && vizeSingle > 0 && vizeMulti > 0) {
  console.log();
  console.log("-".repeat(65));
  console.log();
  console.log(" Summary:");
  console.log();
  const stSpeedup = (vueTscSingle / vizeSingle).toFixed(1);
  const mtSpeedup = (vueTscMulti / vizeMulti).toFixed(1);
  const crossSpeedup = (vueTscSingle / vizeMulti).toFixed(1);
  console.log(`   vue-tsc ST vs Vize ST : ${stSpeedup}x`);
  console.log(`   vue-tsc MT vs Vize MT : ${mtSpeedup}x`);
  console.log(
    `   vue-tsc ST vs Vize MT : ${crossSpeedup}x  (user-facing speedup)`
  );
}

console.log();
