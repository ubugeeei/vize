/**
 * Visual Regression Testing (VRT) module for Musea.
 * Uses Playwright for browser automation and pixel comparison.
 */

import type { Browser, BrowserContext, Page } from "playwright";
import type {
  ArtFileInfo,
  VrtOptions,
  ViewportConfig,
  CaptureConfig,
  ComparisonConfig,
  CiConfig,
} from "./types.js";
import fs from "node:fs";
import path from "node:path";
import { PNG } from "pngjs";

/**
 * VRT test result for a single variant.
 */
export interface VrtResult {
  artPath: string;
  variantName: string;
  viewport: ViewportConfig;
  passed: boolean;
  snapshotPath: string;
  currentPath?: string;
  diffPath?: string;
  diffPercentage?: number;
  diffPixels?: number;
  totalPixels?: number;
  error?: string;
  isNew?: boolean;
}

/**
 * VRT summary for reporting.
 */
export interface VrtSummary {
  total: number;
  passed: number;
  failed: number;
  new: number;
  skipped: number;
  duration: number;
}

/**
 * Extended VRT options aligned with Rust VrtConfig.
 */
export interface ExtendedVrtOptions extends VrtOptions {
  capture?: CaptureConfig;
  comparison?: ComparisonConfig;
  ci?: CiConfig;
  /** Enable a11y auditing during VRT */
  a11y?: boolean;
}

/**
 * Pixel comparison options.
 */
export interface PixelCompareOptions {
  /** Threshold for color difference (0-1). Default: 0.1 */
  threshold?: number;
  /** Include anti-aliasing in diff. Default: false */
  includeAA?: boolean;
  /** Alpha channel comparison. Default: true */
  alpha?: boolean;
  /** Diff highlight color */
  diffColor?: { r: number; g: number; b: number };
}

/**
 * VRT runner using Playwright.
 */
export class MuseaVrtRunner {
  private options: Required<VrtOptions>;
  private capture: Required<CaptureConfig>;
  private comparison: ComparisonConfig;
  private ci: CiConfig;
  private browser: Browser | null = null;
  private startTime: number = 0;

  constructor(options: ExtendedVrtOptions = {}) {
    this.options = {
      snapshotDir: options.snapshotDir ?? ".vize/snapshots",
      threshold: options.threshold ?? 0.1,
      viewports: options.viewports ?? [
        { width: 1280, height: 720, name: "desktop" },
        { width: 375, height: 667, name: "mobile" },
      ],
    };
    this.capture = {
      fullPage: options.capture?.fullPage ?? false,
      waitForNetwork: options.capture?.waitForNetwork ?? true,
      settleTime: options.capture?.settleTime ?? 100,
      waitSelector: options.capture?.waitSelector ?? ".musea-variant",
      hideElements: options.capture?.hideElements ?? [],
      maskElements: options.capture?.maskElements ?? [],
    };
    this.comparison = options.comparison ?? {};
    this.ci = options.ci ?? {};
  }

  /**
   * Initialize Playwright browser.
   */
  async init(): Promise<void> {
    const { chromium } = await import("playwright");
    this.browser = await chromium.launch({ headless: true });
    this.startTime = Date.now();
  }

  /**
   * Close browser and cleanup.
   */
  async close(): Promise<void> {
    if (this.browser) {
      await this.browser.close();
      this.browser = null;
    }
  }

  /**
   * Run VRT tests for all Art files.
   */
  async runAllTests(artFiles: ArtFileInfo[], baseUrl: string): Promise<VrtResult[]> {
    if (!this.browser) {
      throw new Error("VRT runner not initialized. Call init() first.");
    }

    const results: VrtResult[] = [];
    const retries = this.ci.retries ?? 0;

    for (const art of artFiles) {
      for (const variant of art.variants) {
        if (variant.skipVrt) {
          continue;
        }

        // Determine viewports: use per-variant viewport if defined, else global viewports
        const viewports = variant.args?.viewport
          ? [variant.args.viewport as ViewportConfig]
          : this.options.viewports;

        for (const viewport of viewports) {
          let result: VrtResult | null = null;
          let attempts = 0;

          while (attempts <= retries) {
            result = await this.captureAndCompare(art, variant.name, viewport, baseUrl);
            if (result.passed || result.isNew || !result.error) {
              break;
            }
            attempts++;
            if (attempts <= retries) {
              console.log(
                `[vrt] Retry ${attempts}/${retries}: ${path.basename(art.path)}/${variant.name}`,
              );
            }
          }

          if (result) {
            results.push(result);
          }
        }
      }
    }

    return results;
  }

  /**
   * Capture screenshot and compare with baseline.
   */
  async captureAndCompare(
    art: ArtFileInfo,
    variantName: string,
    viewport: ViewportConfig,
    baseUrl: string,
  ): Promise<VrtResult> {
    if (!this.browser) {
      throw new Error("VRT runner not initialized. Call init() first.");
    }

    const snapshotDir = this.options.snapshotDir;
    const artBaseName = path.basename(art.path, ".art.vue");
    const viewportName = viewport.name || `${viewport.width}x${viewport.height}`;
    const snapshotName = `${artBaseName}--${variantName}--${viewportName}.png`;
    const snapshotPath = path.join(snapshotDir, snapshotName);
    const currentPath = path.join(snapshotDir, "current", snapshotName);
    const diffPath = path.join(snapshotDir, "diff", snapshotName);

    // Ensure directories exist
    await fs.promises.mkdir(path.dirname(snapshotPath), { recursive: true });
    await fs.promises.mkdir(path.join(snapshotDir, "current"), { recursive: true });
    await fs.promises.mkdir(path.join(snapshotDir, "diff"), { recursive: true });

    let context: BrowserContext | null = null;
    let page: Page | null = null;

    try {
      context = await this.browser.newContext({
        viewport: {
          width: viewport.width,
          height: viewport.height,
        },
        deviceScaleFactor: viewport.deviceScaleFactor ?? 1,
      });
      page = await context.newPage();

      // Navigate to variant preview URL
      const variantUrl = this.buildVariantUrl(baseUrl, art.path, variantName);
      const waitUntil = this.capture.waitForNetwork ? ("networkidle" as const) : ("load" as const);
      await page.goto(variantUrl, { waitUntil });

      // Wait for content to render
      await page.waitForSelector(this.capture.waitSelector, { timeout: 10000 });

      // Additional wait for animations to settle
      await page.waitForTimeout(this.capture.settleTime);

      // Hide elements before capture
      if (this.capture.hideElements.length > 0) {
        for (const selector of this.capture.hideElements) {
          await page.evaluate((sel) => {
            document.querySelectorAll(sel).forEach((el) => {
              (el as HTMLElement).style.visibility = "hidden";
            });
          }, selector);
        }
      }

      // Mask elements before capture (replace with colored box)
      if (this.capture.maskElements.length > 0) {
        for (const selector of this.capture.maskElements) {
          await page.evaluate((sel) => {
            document.querySelectorAll(sel).forEach((el) => {
              const htmlEl = el as HTMLElement;
              htmlEl.style.background = "#ff00ff";
              htmlEl.style.color = "transparent";
              htmlEl.innerHTML = "";
            });
          }, selector);
        }
      }

      // Take screenshot
      await page.screenshot({
        path: currentPath,
        fullPage: this.capture.fullPage,
      });

      // Check if baseline exists
      const hasBaseline = await fileExists(snapshotPath);

      if (!hasBaseline) {
        // First run - save as baseline
        await fs.promises.copyFile(currentPath, snapshotPath);
        return {
          artPath: art.path,
          variantName,
          viewport,
          passed: true,
          snapshotPath,
          currentPath,
          isNew: true,
        };
      }

      // Compare images using pixel comparison
      const comparison = await this.compareImages(snapshotPath, currentPath, diffPath);

      const passed = comparison.diffPercentage <= this.options.threshold;

      return {
        artPath: art.path,
        variantName,
        viewport,
        passed,
        snapshotPath,
        currentPath,
        diffPath: passed ? undefined : diffPath,
        diffPercentage: comparison.diffPercentage,
        diffPixels: comparison.diffPixels,
        totalPixels: comparison.totalPixels,
      };
    } catch (error) {
      return {
        artPath: art.path,
        variantName,
        viewport,
        passed: false,
        snapshotPath,
        error: error instanceof Error ? error.message : String(error),
      };
    } finally {
      if (page) await page.close();
      if (context) await context.close();
    }
  }

  /**
   * Get the Playwright Page for external use (e.g., a11y auditing).
   */
  async createPage(viewport: ViewportConfig): Promise<{ page: Page; context: BrowserContext }> {
    if (!this.browser) {
      throw new Error("VRT runner not initialized. Call init() first.");
    }
    const context = await this.browser.newContext({
      viewport: { width: viewport.width, height: viewport.height },
      deviceScaleFactor: viewport.deviceScaleFactor ?? 1,
    });
    const page = await context.newPage();
    return { page, context };
  }

  /**
   * Update baseline snapshots with current screenshots.
   */
  async updateBaselines(results: VrtResult[]): Promise<number> {
    let updated = 0;
    const snapshotDir = this.options.snapshotDir;
    const currentDir = path.join(snapshotDir, "current");

    for (const result of results) {
      const currentPath = path.join(currentDir, path.basename(result.snapshotPath));

      if (await fileExists(currentPath)) {
        await fs.promises.copyFile(currentPath, result.snapshotPath);
        updated++;
        console.log(`[vrt] Updated: ${path.basename(result.snapshotPath)}`);
      }
    }

    return updated;
  }

  /**
   * Approve specific failed results (update their baselines).
   */
  async approveResults(results: VrtResult[], pattern?: string): Promise<number> {
    const toApprove = pattern
      ? results.filter((r) => {
          const name = `${path.basename(r.artPath, ".art.vue")}/${r.variantName}`;
          return name.includes(pattern) || matchGlob(name, pattern);
        })
      : results.filter((r) => !r.passed && !r.error);

    return this.updateBaselines(toApprove);
  }

  /**
   * Clean orphaned snapshots (no corresponding art/variant).
   */
  async cleanOrphans(artFiles: ArtFileInfo[]): Promise<number> {
    const snapshotDir = this.options.snapshotDir;
    let cleaned = 0;

    try {
      const files = await fs.promises.readdir(snapshotDir);
      const validNames = new Set<string>();

      for (const art of artFiles) {
        const artBaseName = path.basename(art.path, ".art.vue");
        for (const variant of art.variants) {
          if (variant.skipVrt) continue;
          for (const viewport of this.options.viewports) {
            const viewportName = viewport.name || `${viewport.width}x${viewport.height}`;
            validNames.add(`${artBaseName}--${variant.name}--${viewportName}.png`);
          }
        }
      }

      for (const file of files) {
        if (file.endsWith(".png") && !validNames.has(file)) {
          await fs.promises.unlink(path.join(snapshotDir, file));
          cleaned++;
          console.log(`[vrt] Cleaned: ${file}`);
        }
      }
    } catch {
      // Directory may not exist yet
    }

    return cleaned;
  }

  /**
   * Get VRT summary statistics.
   */
  getSummary(results: VrtResult[]): VrtSummary {
    return {
      total: results.length,
      passed: results.filter((r) => r.passed && !r.isNew).length,
      failed: results.filter((r) => !r.passed && !r.error).length,
      new: results.filter((r) => r.isNew).length,
      skipped: results.filter((r) => r.error).length,
      duration: Date.now() - this.startTime,
    };
  }

  /**
   * Build URL for variant preview.
   */
  private buildVariantUrl(baseUrl: string, artPath: string, variantName: string): string {
    const encodedPath = encodeURIComponent(artPath);
    const encodedVariant = encodeURIComponent(variantName);
    return `${baseUrl}/__musea__/preview?art=${encodedPath}&variant=${encodedVariant}`;
  }

  /**
   * Compare two PNG images and generate a diff image.
   * Returns pixel difference statistics.
   */
  private async compareImages(
    baselinePath: string,
    currentPath: string,
    diffPath: string,
  ): Promise<{ diffPixels: number; totalPixels: number; diffPercentage: number }> {
    const baseline = await readPng(baselinePath);
    const current = await readPng(currentPath);

    // Handle size mismatch
    if (baseline.width !== current.width || baseline.height !== current.height) {
      // Create a diff showing the size mismatch
      const width = Math.max(baseline.width, current.width);
      const height = Math.max(baseline.height, current.height);
      const diff = new PNG({ width, height });

      // Fill with red to indicate size mismatch
      for (let i = 0; i < diff.data.length; i += 4) {
        diff.data[i] = 255; // R
        diff.data[i + 1] = 0; // G
        diff.data[i + 2] = 0; // B
        diff.data[i + 3] = 255; // A
      }

      await writePng(diff, diffPath);

      return {
        diffPixels: width * height,
        totalPixels: width * height,
        diffPercentage: 100,
      };
    }

    const width = baseline.width;
    const height = baseline.height;
    const totalPixels = width * height;
    const diff = new PNG({ width, height });

    const useAntiAliasing = this.comparison.antiAliasing ?? true;
    const useAlpha = this.comparison.alpha ?? true;
    const diffColor = this.comparison.diffColor ?? { r: 255, g: 0, b: 0 };

    // Pixel comparison
    let diffPixels = 0;
    const threshold = 0.1; // Color difference threshold

    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const idx = (y * width + x) * 4;

        const r1 = baseline.data[idx];
        const g1 = baseline.data[idx + 1];
        const b1 = baseline.data[idx + 2];
        const a1 = useAlpha ? baseline.data[idx + 3] : 255;

        const r2 = current.data[idx];
        const g2 = current.data[idx + 1];
        const b2 = current.data[idx + 2];
        const a2 = useAlpha ? current.data[idx + 3] : 255;

        // Calculate color difference using YIQ color space
        const delta = colorDelta(r1, g1, b1, a1, r2, g2, b2, a2);

        if (delta > threshold * 255 * 255) {
          // Anti-aliasing detection: check if pixel is likely AA
          if (useAntiAliasing && isAntiAliased(baseline, current, x, y, width, height)) {
            // Mark as AA (yellow)
            diff.data[idx] = 255;
            diff.data[idx + 1] = 200;
            diff.data[idx + 2] = 0;
            diff.data[idx + 3] = 128;
          } else {
            // Mark as different
            diffPixels++;
            diff.data[idx] = diffColor.r;
            diff.data[idx + 1] = diffColor.g;
            diff.data[idx + 2] = diffColor.b;
            diff.data[idx + 3] = 255;
          }
        } else {
          // Grayscale for matching pixels
          const gray = Math.round((r2 + g2 + b2) / 3);
          diff.data[idx] = gray;
          diff.data[idx + 1] = gray;
          diff.data[idx + 2] = gray;
          diff.data[idx + 3] = 128; // Semi-transparent
        }
      }
    }

    // Only write diff if there are differences
    if (diffPixels > 0) {
      await writePng(diff, diffPath);
    }

    const diffPercentage = (diffPixels / totalPixels) * 100;

    return {
      diffPixels,
      totalPixels,
      diffPercentage,
    };
  }
}

/**
 * Generate VRT report in HTML format.
 * Supports side-by-side, overlay, and slider comparison modes.
 */
export function generateVrtReport(results: VrtResult[], summary: VrtSummary): string {
  const formatDuration = (ms: number): string => {
    if (ms < 1000) return `${ms}ms`;
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    if (minutes === 0) return `${seconds}s`;
    return `${minutes}m ${seconds % 60}s`;
  };

  const timestamp = new Date().toLocaleString("ja-JP", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });

  const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>VRT Report - Musea</title>
  <style>
    :root {
      --musea-bg-primary: #0d0d0d;
      --musea-bg-secondary: #1a1815;
      --musea-bg-tertiary: #252220;
      --musea-accent: #a34828;
      --musea-accent-hover: #c45a32;
      --musea-text: #e6e9f0;
      --musea-text-muted: #7b8494;
      --musea-border: #3a3530;
      --musea-success: #4ade80;
      --musea-error: #f87171;
      --musea-info: #60a5fa;
      --musea-warning: #fbbf24;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
      background: var(--musea-bg-primary);
      color: var(--musea-text);
      min-height: 100vh;
      line-height: 1.5;
    }

    .header {
      background: var(--musea-bg-secondary);
      border-bottom: 1px solid var(--musea-border);
      padding: 1rem 2rem;
      display: flex;
      align-items: center;
      justify-content: space-between;
      position: sticky;
      top: 0;
      z-index: 100;
    }
    .header-left { display: flex; align-items: center; gap: 1rem; }
    .logo { font-size: 1.25rem; font-weight: 700; color: var(--musea-accent); }
    .header-title { color: var(--musea-text-muted); font-size: 0.875rem; }
    .header-meta { display: flex; align-items: center; gap: 1.5rem; font-size: 0.8125rem; color: var(--musea-text-muted); }
    .header-meta span { display: flex; align-items: center; gap: 0.375rem; }

    .main { max-width: 1400px; margin: 0 auto; padding: 2rem; }

    .summary { display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 1rem; margin-bottom: 2rem; }
    .stat { background: var(--musea-bg-secondary); border: 1px solid var(--musea-border); border-radius: 8px; padding: 1.25rem; position: relative; overflow: hidden; }
    .stat::before { content: ''; position: absolute; left: 0; top: 0; bottom: 0; width: 3px; }
    .stat.passed::before { background: var(--musea-success); }
    .stat.failed::before { background: var(--musea-error); }
    .stat.new::before { background: var(--musea-info); }
    .stat.skipped::before { background: var(--musea-warning); }
    .stat-value { font-size: 2rem; font-weight: 700; font-variant-numeric: tabular-nums; line-height: 1; margin-bottom: 0.25rem; }
    .stat.passed .stat-value { color: var(--musea-success); }
    .stat.failed .stat-value { color: var(--musea-error); }
    .stat.new .stat-value { color: var(--musea-info); }
    .stat.skipped .stat-value { color: var(--musea-warning); }
    .stat-label { color: var(--musea-text-muted); font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.08em; font-weight: 500; }

    .filters { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; flex-wrap: wrap; }
    .filter-btn { background: var(--musea-bg-secondary); border: 1px solid var(--musea-border); color: var(--musea-text); padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer; font-size: 0.8125rem; font-weight: 500; transition: all 0.15s ease; }
    .filter-btn:hover { background: var(--musea-bg-tertiary); border-color: var(--musea-text-muted); }
    .filter-btn.active { background: var(--musea-accent); border-color: var(--musea-accent); color: #fff; }
    .filter-btn .count { opacity: 0.7; margin-left: 0.25rem; }

    /* Comparison mode toggle */
    .compare-modes { display: flex; gap: 0.25rem; margin-bottom: 1.5rem; background: var(--musea-bg-secondary); border-radius: 6px; padding: 0.25rem; width: fit-content; }
    .compare-mode-btn { background: none; border: none; color: var(--musea-text-muted); padding: 0.375rem 0.75rem; border-radius: 4px; cursor: pointer; font-size: 0.75rem; font-weight: 500; transition: all 0.15s ease; }
    .compare-mode-btn.active { background: var(--musea-bg-tertiary); color: var(--musea-text); }

    .results { display: flex; flex-direction: column; gap: 0.75rem; }
    .result { background: var(--musea-bg-secondary); border: 1px solid var(--musea-border); border-radius: 8px; overflow: hidden; transition: border-color 0.15s ease; }
    .result:hover { border-color: var(--musea-text-muted); }
    .result-header { padding: 1rem 1.25rem; display: flex; justify-content: space-between; align-items: center; cursor: pointer; border-left: 3px solid transparent; background: var(--musea-bg-tertiary); }
    .result.passed .result-header { border-left-color: var(--musea-success); }
    .result.failed .result-header { border-left-color: var(--musea-error); }
    .result.new .result-header { border-left-color: var(--musea-info); }
    .result.error .result-header { border-left-color: var(--musea-warning); }

    .result-info { display: flex; align-items: center; gap: 1rem; }
    .result-name { font-weight: 600; font-size: 0.9375rem; }
    .result-meta { color: var(--musea-text-muted); font-size: 0.8125rem; padding: 0.125rem 0.5rem; background: var(--musea-bg-secondary); border-radius: 4px; }
    .result-badge { padding: 0.25rem 0.625rem; border-radius: 4px; font-size: 0.6875rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em; }
    .result.passed .result-badge { background: rgba(74, 222, 128, 0.15); color: var(--musea-success); }
    .result.failed .result-badge { background: rgba(248, 113, 113, 0.15); color: var(--musea-error); }
    .result.new .result-badge { background: rgba(96, 165, 250, 0.15); color: var(--musea-info); }
    .result.error .result-badge { background: rgba(251, 191, 36, 0.15); color: var(--musea-warning); }

    .result-body { border-top: 1px solid var(--musea-border); }
    .result-details { padding: 0.875rem 1.25rem; font-size: 0.8125rem; color: var(--musea-text-muted); font-family: 'SF Mono', 'Fira Code', monospace; background: var(--musea-bg-primary); }
    .result-details.error { color: var(--musea-error); }

    .result-images { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1rem; padding: 1.25rem; background: var(--musea-bg-primary); }
    .result-images.overlay { grid-template-columns: 1fr; }
    .image-container { background: var(--musea-bg-secondary); border: 1px solid var(--musea-border); border-radius: 6px; overflow: hidden; }
    .image-label { padding: 0.625rem 0.875rem; font-size: 0.6875rem; font-weight: 600; color: var(--musea-text-muted); text-transform: uppercase; letter-spacing: 0.08em; background: var(--musea-bg-tertiary); border-bottom: 1px solid var(--musea-border); }
    .image-wrapper { padding: 0.5rem; background: repeating-conic-gradient(var(--musea-bg-tertiary) 0% 25%, var(--musea-bg-secondary) 0% 50%) 50% / 16px 16px; }
    .image-container img { width: 100%; height: auto; display: block; border-radius: 2px; }

    /* Slider comparison */
    .slider-compare { position: relative; overflow: hidden; }
    .slider-compare img { display: block; width: 100%; }
    .slider-overlay { position: absolute; top: 0; left: 0; bottom: 0; overflow: hidden; border-right: 2px solid var(--musea-accent); }
    .slider-overlay img { display: block; min-width: 100%; height: 100%; object-fit: cover; }

    .empty-state { text-align: center; padding: 4rem 2rem; color: var(--musea-text-muted); }
    .all-passed { background: rgba(74, 222, 128, 0.1); border: 1px solid rgba(74, 222, 128, 0.2); border-radius: 8px; padding: 1.5rem; text-align: center; margin-bottom: 1.5rem; }
    .all-passed-icon { font-size: 2.5rem; margin-bottom: 0.5rem; }
    .all-passed-text { color: var(--musea-success); font-weight: 600; }
  </style>
</head>
<body>
  <header class="header">
    <div class="header-left">
      <div class="logo">Musea</div>
      <span class="header-title">Visual Regression Report</span>
    </div>
    <div class="header-meta">
      <span>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>
        </svg>
        ${formatDuration(summary.duration)}
      </span>
      <span>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/>
        </svg>
        ${timestamp}
      </span>
    </div>
  </header>

  <main class="main">
    <div class="summary">
      <div class="stat passed"><div class="stat-value">${summary.passed}</div><div class="stat-label">Passed</div></div>
      <div class="stat failed"><div class="stat-value">${summary.failed}</div><div class="stat-label">Failed</div></div>
      <div class="stat new"><div class="stat-value">${summary.new}</div><div class="stat-label">New</div></div>
      <div class="stat skipped"><div class="stat-value">${summary.skipped}</div><div class="stat-label">Skipped</div></div>
    </div>

    ${
      summary.failed === 0 && summary.skipped === 0 && summary.total > 0
        ? `<div class="all-passed">
            <div class="all-passed-icon">✓</div>
            <div class="all-passed-text">All ${summary.total} visual tests passed</div>
          </div>`
        : ""
    }

    <div class="filters">
      <button class="filter-btn active" data-filter="all">All<span class="count">(${summary.total})</span></button>
      <button class="filter-btn" data-filter="failed">Failed<span class="count">(${summary.failed})</span></button>
      <button class="filter-btn" data-filter="passed">Passed<span class="count">(${summary.passed})</span></button>
      <button class="filter-btn" data-filter="new">New<span class="count">(${summary.new})</span></button>
    </div>

    <div class="compare-modes">
      <button class="compare-mode-btn active" data-mode="side-by-side">Side by Side</button>
      <button class="compare-mode-btn" data-mode="overlay">Overlay</button>
      <button class="compare-mode-btn" data-mode="slider">Slider</button>
    </div>

    <div class="results">
      ${
        results.length === 0
          ? `<div class="empty-state"><p>No visual tests found</p></div>`
          : results
              .map((r) => {
                const status = r.error ? "error" : r.isNew ? "new" : r.passed ? "passed" : "failed";
                const badge = r.error ? "Error" : r.isNew ? "New" : r.passed ? "Passed" : "Failed";
                const artName = path.basename(r.artPath, ".art.vue");
                const viewportName = r.viewport.name || `${r.viewport.width}×${r.viewport.height}`;

                let details = "";
                if (r.error) {
                  details = `<div class="result-details error">${escapeHtml(r.error)}</div>`;
                } else if (r.diffPercentage !== undefined) {
                  const diffFormatted = r.diffPercentage.toFixed(3);
                  const pixelsFormatted = r.diffPixels?.toLocaleString() ?? "0";
                  const totalFormatted = r.totalPixels?.toLocaleString() ?? "0";
                  details = `<div class="result-details">diff: ${diffFormatted}% (${pixelsFormatted} / ${totalFormatted} pixels)</div>`;
                }

                let images = "";
                if (!r.error && !r.passed && r.diffPath) {
                  images = `<div class="result-images" data-baseline="file://${r.snapshotPath}" data-current="file://${r.currentPath}" data-diff="file://${r.diffPath}">
                    ${r.snapshotPath ? `<div class="image-container"><div class="image-label">Baseline</div><div class="image-wrapper"><img src="file://${r.snapshotPath}" alt="Baseline" loading="lazy" /></div></div>` : ""}
                    ${r.currentPath ? `<div class="image-container"><div class="image-label">Current</div><div class="image-wrapper"><img src="file://${r.currentPath}" alt="Current" loading="lazy" /></div></div>` : ""}
                    ${r.diffPath ? `<div class="image-container"><div class="image-label">Diff</div><div class="image-wrapper"><img src="file://${r.diffPath}" alt="Diff" loading="lazy" /></div></div>` : ""}
                  </div>`;
                }

                const hasBody = details || images;

                return `<div class="result ${status}" data-status="${status}">
                  <div class="result-header">
                    <div class="result-info">
                      <div class="result-name">${escapeHtml(artName)} / ${escapeHtml(r.variantName)}</div>
                      <div class="result-meta">${escapeHtml(viewportName)}</div>
                    </div>
                    <span class="result-badge">${badge}</span>
                  </div>
                  ${hasBody ? `<div class="result-body">${details}${images}</div>` : ""}
                </div>`;
              })
              .join("")
      }
    </div>
  </main>

  <script>
    // Filter buttons
    document.querySelectorAll('.filter-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        document.querySelectorAll('.filter-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        const filter = btn.dataset.filter;
        document.querySelectorAll('.result').forEach(result => {
          result.style.display = (filter === 'all' || result.dataset.status === filter) ? 'block' : 'none';
        });
      });
    });

    // Compare mode buttons
    document.querySelectorAll('.compare-mode-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        document.querySelectorAll('.compare-mode-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        // Mode switching would update result-images display; this is a static report for now
      });
    });
  </script>
</body>
</html>`;

  return html;
}

/**
 * Generate VRT JSON report for CI integration.
 */
export function generateVrtJsonReport(results: VrtResult[], summary: VrtSummary): string {
  return JSON.stringify(
    {
      timestamp: new Date().toISOString(),
      summary,
      results: results.map((r) => ({
        art: path.basename(r.artPath, ".art.vue"),
        variant: r.variantName,
        viewport: r.viewport.name || `${r.viewport.width}x${r.viewport.height}`,
        status: r.error ? "error" : r.isNew ? "new" : r.passed ? "passed" : "failed",
        diffPercentage: r.diffPercentage,
        error: r.error,
      })),
    },
    null,
    2,
  );
}

// Utility functions

/**
 * Read PNG file and return PNG object.
 */
async function readPng(filepath: string): Promise<PNG> {
  return new Promise((resolve, reject) => {
    fs.createReadStream(filepath)
      .pipe(new PNG())
      .on("parsed", function (this: PNG) {
        resolve(this);
      })
      .on("error", reject);
  });
}

/**
 * Write PNG object to file.
 */
async function writePng(png: PNG, filepath: string): Promise<void> {
  return new Promise((resolve, reject) => {
    png.pack().pipe(fs.createWriteStream(filepath)).on("finish", resolve).on("error", reject);
  });
}

/**
 * Calculate color delta using YIQ color space.
 */
function colorDelta(
  r1: number,
  g1: number,
  b1: number,
  a1: number,
  r2: number,
  g2: number,
  b2: number,
  a2: number,
): number {
  if (a1 !== 255) {
    r1 = blend(r1, 255, a1 / 255);
    g1 = blend(g1, 255, a1 / 255);
    b1 = blend(b1, 255, a1 / 255);
  }
  if (a2 !== 255) {
    r2 = blend(r2, 255, a2 / 255);
    g2 = blend(g2, 255, a2 / 255);
    b2 = blend(b2, 255, a2 / 255);
  }

  const y1 = r1 * 0.29889531 + g1 * 0.58662247 + b1 * 0.11448223;
  const i1 = r1 * 0.59597799 - g1 * 0.2741761 - b1 * 0.32180189;
  const q1 = r1 * 0.21147017 - g1 * 0.52261711 + b1 * 0.31114694;

  const y2 = r2 * 0.29889531 + g2 * 0.58662247 + b2 * 0.11448223;
  const i2 = r2 * 0.59597799 - g2 * 0.2741761 - b2 * 0.32180189;
  const q2 = r2 * 0.21147017 - g2 * 0.52261711 + b2 * 0.31114694;

  const dy = y1 - y2;
  const di = i1 - i2;
  const dq = q1 - q2;

  return dy * dy * 0.5053 + di * di * 0.299 + dq * dq * 0.1957;
}

/**
 * Blend foreground with background using alpha.
 */
function blend(fg: number, bg: number, alpha: number): number {
  return bg + (fg - bg) * alpha;
}

/**
 * Check if file exists.
 */
async function fileExists(filepath: string): Promise<boolean> {
  try {
    await fs.promises.access(filepath);
    return true;
  } catch {
    return false;
  }
}

/**
 * Simple anti-aliasing detection.
 * A pixel is likely anti-aliased if its neighbors have high contrast in opposite directions.
 */
function isAntiAliased(
  img1: PNG,
  img2: PNG,
  x: number,
  y: number,
  width: number,
  height: number,
): boolean {
  const minX = Math.max(0, x - 1);
  const maxX = Math.min(width - 1, x + 1);
  const minY = Math.max(0, y - 1);
  const maxY = Math.min(height - 1, y + 1);

  let zeroes = 0;
  let positives = 0;
  let negatives = 0;

  for (let ny = minY; ny <= maxY; ny++) {
    for (let nx = minX; nx <= maxX; nx++) {
      if (nx === x && ny === y) continue;
      const idx = (ny * width + nx) * 4;

      const delta = colorDelta(
        img1.data[idx],
        img1.data[idx + 1],
        img1.data[idx + 2],
        img1.data[idx + 3],
        img2.data[idx],
        img2.data[idx + 1],
        img2.data[idx + 2],
        img2.data[idx + 3],
      );

      if (delta === 0) {
        zeroes++;
      } else if (delta > 0) {
        positives++;
      } else {
        negatives++;
      }
    }
  }

  // If neighbors are mixed (some match, some differ), it's likely AA
  return zeroes > 0 && (positives > 0 || negatives > 0) && positives + negatives < 4;
}

/**
 * Simple glob matching for pattern-based filtering.
 */
function matchGlob(filepath: string, pattern: string): boolean {
  const regex = pattern
    .replace(/\./g, "\\.")
    .replace(/\*\*/g, ".*")
    .replace(/\*(?!\*)/g, "[^/]*");
  return new RegExp(`^${regex}$`).test(filepath);
}

/**
 * Escape HTML special characters.
 */
function escapeHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#x27;");
}

export default MuseaVrtRunner;
