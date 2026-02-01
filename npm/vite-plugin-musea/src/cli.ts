#!/usr/bin/env node
/**
 * Musea VRT CLI
 *
 * Usage:
 *   musea-vrt [options]
 *
 * Options:
 *   -u, --update     Update baseline snapshots
 *   -c, --config     Path to vite config (default: vite.config.ts)
 *   -o, --output     Output directory for reports (default: .vize)
 *   -t, --threshold  Diff threshold percentage (default: 0.1)
 *   --json           Output JSON report instead of HTML
 *   --ci             CI mode - exit with non-zero code on failures
 *   -h, --help       Show help
 */

import fs from "node:fs";
import path from "node:path";
import { MuseaVrtRunner, generateVrtReport, generateVrtJsonReport } from "./vrt.js";
import type { ArtFileInfo, VrtOptions } from "./types.js";

interface CliOptions {
  update: boolean;
  config: string;
  output: string;
  threshold: number;
  json: boolean;
  ci: boolean;
  help: boolean;
  baseUrl: string;
}

function parseArgs(args: string[]): CliOptions {
  const options: CliOptions = {
    update: false,
    config: "vite.config.ts",
    output: ".vize",
    threshold: 0.1,
    json: false,
    ci: false,
    help: false,
    baseUrl: "http://localhost:5173",
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    switch (arg) {
      case "-u":
      case "--update":
        options.update = true;
        break;
      case "-c":
      case "--config":
        options.config = args[++i] || "vite.config.ts";
        break;
      case "-o":
      case "--output":
        options.output = args[++i] || ".vize";
        break;
      case "-t":
      case "--threshold":
        options.threshold = parseFloat(args[++i]) || 0.1;
        break;
      case "--json":
        options.json = true;
        break;
      case "--ci":
        options.ci = true;
        break;
      case "-b":
      case "--base-url":
        options.baseUrl = args[++i] || "http://localhost:5173";
        break;
      case "-h":
      case "--help":
        options.help = true;
        break;
    }
  }

  return options;
}

function printHelp(): void {
  console.log(`
Musea VRT - Visual Regression Testing for Component Gallery

Usage:
  musea-vrt [options]

Options:
  -u, --update         Update baseline snapshots with current screenshots
  -c, --config <path>  Path to vite config file (default: vite.config.ts)
  -o, --output <dir>   Output directory for reports (default: .vize)
  -t, --threshold <n>  Diff threshold percentage (default: 0.1)
  -b, --base-url <url> Base URL for dev server (default: http://localhost:5173)
  --json               Output JSON report instead of HTML
  --ci                 CI mode - exit with non-zero code on failures
  -h, --help           Show this help message

Examples:
  # Run VRT tests
  musea-vrt

  # Update baseline snapshots
  musea-vrt -u

  # Run with custom threshold
  musea-vrt -t 0.5

  # CI mode with JSON output
  musea-vrt --ci --json

  # Custom base URL
  musea-vrt -b http://localhost:3000
`);
}

async function scanArtFiles(root: string): Promise<string[]> {
  const files: string[] = [];

  async function scan(dir: string): Promise<void> {
    const entries = await fs.promises.readdir(dir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);

      // Skip node_modules and dist
      if (entry.name === "node_modules" || entry.name === "dist") {
        continue;
      }

      if (entry.isDirectory()) {
        await scan(fullPath);
      } else if (entry.isFile() && entry.name.endsWith(".art.vue")) {
        files.push(fullPath);
      }
    }
  }

  await scan(root);
  return files;
}

async function parseArtFile(filePath: string): Promise<ArtFileInfo | null> {
  try {
    const source = await fs.promises.readFile(filePath, "utf-8");

    // Simple parsing - in production, use @vizejs/native
    const titleMatch = source.match(/<art[^>]*\stitle=["']([^"']+)["']/);
    const componentMatch = source.match(/<art[^>]*\scomponent=["']([^"']+)["']/);
    const categoryMatch = source.match(/<art[^>]*\scategory=["']([^"']+)["']/);

    const variants: ArtFileInfo["variants"] = [];
    const variantRegex = /<variant\s+([^>]*)>([\s\S]*?)<\/variant>/g;
    let match;

    while ((match = variantRegex.exec(source)) !== null) {
      const attrs = match[1];
      const template = match[2].trim();

      const nameMatch = attrs.match(/name=["']([^"']+)["']/);
      const isDefault = /\bdefault\b/.test(attrs);
      const skipVrt = /\bskip-vrt\b/.test(attrs);

      if (nameMatch) {
        variants.push({
          name: nameMatch[1],
          template,
          isDefault,
          skipVrt,
        });
      }
    }

    return {
      path: filePath,
      metadata: {
        title: titleMatch?.[1] || path.basename(filePath, ".art.vue"),
        component: componentMatch?.[1],
        category: categoryMatch?.[1],
        tags: [],
        status: "ready",
      },
      variants,
      hasScriptSetup: /<script\s+setup/.test(source),
      hasScript: /<script(?!\s+setup)/.test(source),
      styleCount: (source.match(/<style/g) || []).length,
    };
  } catch (error) {
    console.error(`Failed to parse ${filePath}:`, error);
    return null;
  }
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const options = parseArgs(args);

  if (options.help) {
    printHelp();
    process.exit(0);
  }

  const cwd = process.cwd();

  console.log("\n  Musea VRT");
  console.log("  =========\n");

  // Scan for art files
  console.log("  Scanning for art files...");
  const artFilePaths = await scanArtFiles(cwd);

  if (artFilePaths.length === 0) {
    console.log("  No art files found.\n");
    process.exit(0);
  }

  console.log(`  Found ${artFilePaths.length} art file(s)\n`);

  // Parse art files
  const artFiles: ArtFileInfo[] = [];
  for (const filePath of artFilePaths) {
    const art = await parseArtFile(filePath);
    if (art) {
      artFiles.push(art);
    }
  }

  // Count total variants
  const totalVariants = artFiles.reduce(
    (sum, art) => sum + art.variants.filter((v) => !v.skipVrt).length,
    0,
  );

  console.log(`  Testing ${totalVariants} variant(s) across ${artFiles.length} art file(s)\n`);

  // Initialize VRT runner
  const vrtOptions: VrtOptions = {
    snapshotDir: path.join(options.output, "snapshots"),
    threshold: options.threshold,
  };

  const runner = new MuseaVrtRunner(vrtOptions);

  try {
    console.log("  Launching browser...");
    await runner.init();

    console.log("  Running visual regression tests...\n");

    // Run tests
    const results = await runner.runAllTests(artFiles, options.baseUrl);
    const summary = runner.getSummary(results);

    // Print results
    console.log("  Results:");
    console.log(`  ---------`);
    console.log(`    Passed:  ${summary.passed}`);
    console.log(`    Failed:  ${summary.failed}`);
    console.log(`    New:     ${summary.new}`);
    console.log(`    Skipped: ${summary.skipped}`);
    console.log(`    Total:   ${summary.total}`);
    console.log(`    Duration: ${(summary.duration / 1000).toFixed(2)}s\n`);

    // Update baselines if requested
    if (options.update) {
      console.log("  Updating baselines...");
      const updated = await runner.updateBaselines(results);
      console.log(`  Updated ${updated} baseline(s)\n`);
    }

    // Generate report
    const reportDir = options.output;
    await fs.promises.mkdir(reportDir, { recursive: true });

    if (options.json) {
      const jsonReport = generateVrtJsonReport(results, summary);
      const jsonPath = path.join(reportDir, "vrt-report.json");
      await fs.promises.writeFile(jsonPath, jsonReport);
      console.log(`  JSON report: ${jsonPath}\n`);
    } else {
      const htmlReport = generateVrtReport(results, summary);
      const htmlPath = path.join(reportDir, "vrt-report.html");
      await fs.promises.writeFile(htmlPath, htmlReport);
      console.log(`  HTML report: ${htmlPath}\n`);
    }

    // CI mode - exit with error if failures
    if (options.ci && summary.failed > 0) {
      console.log("  CI mode: Exiting with error due to failures\n");
      process.exit(1);
    }
  } catch (error) {
    console.error("\n  Error:", error);
    process.exit(1);
  } finally {
    await runner.close();
  }
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
