/**
 * Accessibility (a11y) testing module for Musea.
 * Uses axe-core for automated accessibility auditing via Playwright.
 */

import type { Page } from "playwright";
import type {
  ArtFileInfo,
  A11yResult,
  A11yViolation,
  A11yOptions,
  ViewportConfig,
} from "./types.js";
import type { MuseaVrtRunner } from "./vrt.js";
import path from "node:path";

/**
 * A11y audit summary.
 */
export interface A11ySummary {
  totalComponents: number;
  totalVariants: number;
  totalViolations: number;
  criticalCount: number;
  seriousCount: number;
  moderateCount: number;
  minorCount: number;
}

/**
 * axe-core result shape (subset).
 */
interface AxeResult {
  violations: Array<{
    id: string;
    impact: string;
    description: string;
    helpUrl: string;
    nodes: Array<unknown>;
  }>;
  passes: Array<unknown>;
  incomplete: Array<unknown>;
}

/**
 * A11y runner using axe-core via Playwright.
 */
export class MuseaA11yRunner {
  private options: Required<A11yOptions>;

  constructor(options: A11yOptions = {}) {
    this.options = {
      enabled: options.enabled ?? true,
      includeRules: options.includeRules ?? [],
      excludeRules: options.excludeRules ?? [],
      level: options.level ?? "AA",
    };
  }

  /**
   * Run a11y audits on all art file variants.
   * Reuses VRT runner's browser if available.
   */
  async runAudits(
    artFiles: ArtFileInfo[],
    baseUrl: string,
    vrtRunner?: MuseaVrtRunner,
  ): Promise<A11yResult[]> {
    const results: A11yResult[] = [];
    const defaultViewport: ViewportConfig = { width: 1280, height: 720, name: "desktop" };

    for (const art of artFiles) {
      for (const variant of art.variants) {
        if (variant.skipVrt) continue;

        let page: Page | null = null;
        let context: { page: Page; context: { close(): Promise<void> } } | null = null;

        try {
          if (vrtRunner) {
            context = await vrtRunner.createPage(defaultViewport);
            page = context.page;
          } else {
            // Standalone mode: launch own browser
            const { chromium } = await import("playwright");
            const browser = await chromium.launch({ headless: true });
            const ctx = await browser.newContext({
              viewport: { width: defaultViewport.width, height: defaultViewport.height },
            });
            page = await ctx.newPage();
            context = { page, context: ctx };
          }

          const variantUrl = this.buildVariantUrl(baseUrl, art.path, variant.name);
          await page.goto(variantUrl, { waitUntil: "networkidle" });
          await page.waitForSelector(".musea-variant", { timeout: 10000 });
          await page.waitForTimeout(200);

          const result = await this.auditPage(page, art.path, variant.name);
          results.push(result);
        } catch (error) {
          results.push({
            artPath: art.path,
            variantName: variant.name,
            violations: [
              {
                id: "audit-error",
                impact: "critical",
                description: `Audit failed: ${error instanceof Error ? error.message : String(error)}`,
                helpUrl: "",
                nodes: 0,
              },
            ],
            passes: 0,
            incomplete: 0,
          });
        } finally {
          if (context) {
            await context.context.close();
          }
        }
      }
    }

    return results;
  }

  /**
   * Audit a single page using axe-core.
   */
  async auditPage(page: Page, artPath: string, variantName: string): Promise<A11yResult> {
    // Inject axe-core into the page
    const axeSource = await this.getAxeSource();
    await page.evaluate(axeSource);

    // Build axe-core run options
    const runOptions = this.buildAxeOptions();

    // Run axe-core
    const axeResult = (await page.evaluate((opts) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      return (window as any).axe.run(document, opts);
    }, runOptions)) as AxeResult;

    // Map to our result format
    const violations: A11yViolation[] = axeResult.violations.map((v) => ({
      id: v.id,
      impact: v.impact as A11yViolation["impact"],
      description: v.description,
      helpUrl: v.helpUrl,
      nodes: v.nodes.length,
    }));

    return {
      artPath,
      variantName,
      violations,
      passes: axeResult.passes.length,
      incomplete: axeResult.incomplete.length,
    };
  }

  /**
   * Get summary statistics from results.
   */
  getSummary(results: A11yResult[]): A11ySummary {
    const components = new Set(results.map((r) => r.artPath));
    const allViolations = results.flatMap((r) => r.violations);

    return {
      totalComponents: components.size,
      totalVariants: results.length,
      totalViolations: allViolations.length,
      criticalCount: allViolations.filter((v) => v.impact === "critical").length,
      seriousCount: allViolations.filter((v) => v.impact === "serious").length,
      moderateCount: allViolations.filter((v) => v.impact === "moderate").length,
      minorCount: allViolations.filter((v) => v.impact === "minor").length,
    };
  }

  /**
   * Generate HTML report.
   */
  generateHtmlReport(results: A11yResult[]): string {
    const summary = this.getSummary(results);
    const timestamp = new Date().toLocaleString("ja-JP", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });

    const impactColor = (impact: string): string => {
      switch (impact) {
        case "critical":
          return "#f87171";
        case "serious":
          return "#fb923c";
        case "moderate":
          return "#fbbf24";
        case "minor":
          return "#60a5fa";
        default:
          return "#7b8494";
      }
    };

    const resultItems = results
      .filter((r) => r.violations.length > 0)
      .map((r) => {
        const artName = path.basename(r.artPath, ".art.vue");
        const violationRows = r.violations
          .map(
            (v) => `
            <tr>
              <td><span style="color:${impactColor(v.impact)};font-weight:600;text-transform:uppercase;font-size:0.6875rem">${escapeHtml(v.impact)}</span></td>
              <td><code>${escapeHtml(v.id)}</code></td>
              <td>${escapeHtml(v.description)}</td>
              <td>${v.nodes}</td>
              <td>${v.helpUrl ? `<a href="${escapeHtml(v.helpUrl)}" target="_blank" style="color:#60a5fa">docs</a>` : ""}</td>
            </tr>`,
          )
          .join("");

        return `
        <div class="result">
          <div class="result-header">
            <div class="result-info">
              <span class="result-name">${escapeHtml(artName)} / ${escapeHtml(r.variantName)}</span>
              <span class="result-count">${r.violations.length} violation(s)</span>
            </div>
          </div>
          <table class="violations-table">
            <thead><tr><th>Impact</th><th>Rule</th><th>Description</th><th>Nodes</th><th>Help</th></tr></thead>
            <tbody>${violationRows}</tbody>
          </table>
        </div>`;
      })
      .join("");

    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>A11y Report - Musea</title>
  <style>
    :root {
      --musea-bg-primary: #0d0d0d;
      --musea-bg-secondary: #1a1815;
      --musea-bg-tertiary: #252220;
      --musea-accent: #a34828;
      --musea-text: #e6e9f0;
      --musea-text-muted: #7b8494;
      --musea-border: #3a3530;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
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
    }
    .logo { font-size: 1.25rem; font-weight: 700; color: var(--musea-accent); }
    .header-meta { color: var(--musea-text-muted); font-size: 0.8125rem; }
    .main { max-width: 1200px; margin: 0 auto; padding: 2rem; }
    .summary { display: grid; grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); gap: 1rem; margin-bottom: 2rem; }
    .stat { background: var(--musea-bg-secondary); border: 1px solid var(--musea-border); border-radius: 8px; padding: 1rem; text-align: center; }
    .stat-value { font-size: 1.75rem; font-weight: 700; font-variant-numeric: tabular-nums; }
    .stat-label { color: var(--musea-text-muted); font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.08em; }
    .stat.critical .stat-value { color: #f87171; }
    .stat.serious .stat-value { color: #fb923c; }
    .stat.moderate .stat-value { color: #fbbf24; }
    .stat.minor .stat-value { color: #60a5fa; }
    .stat.total .stat-value { color: var(--musea-text); }
    .results { display: flex; flex-direction: column; gap: 1rem; }
    .result { background: var(--musea-bg-secondary); border: 1px solid var(--musea-border); border-radius: 8px; overflow: hidden; }
    .result-header { padding: 1rem; background: var(--musea-bg-tertiary); display: flex; justify-content: space-between; align-items: center; }
    .result-name { font-weight: 600; }
    .result-count { color: var(--musea-text-muted); font-size: 0.8125rem; }
    .violations-table { width: 100%; border-collapse: collapse; font-size: 0.8125rem; }
    .violations-table th { padding: 0.75rem 1rem; text-align: left; color: var(--musea-text-muted); font-weight: 500; font-size: 0.6875rem; text-transform: uppercase; letter-spacing: 0.08em; border-bottom: 1px solid var(--musea-border); }
    .violations-table td { padding: 0.75rem 1rem; border-bottom: 1px solid var(--musea-border); }
    .violations-table code { background: var(--musea-bg-tertiary); padding: 0.125rem 0.375rem; border-radius: 3px; font-size: 0.75rem; }
    .all-clear { background: rgba(74, 222, 128, 0.1); border: 1px solid rgba(74, 222, 128, 0.2); border-radius: 8px; padding: 2rem; text-align: center; }
    .all-clear-text { color: #4ade80; font-weight: 600; }
  </style>
</head>
<body>
  <header class="header">
    <div><span class="logo">Musea</span> <span style="color:var(--musea-text-muted);font-size:0.875rem;margin-left:1rem">Accessibility Report</span></div>
    <div class="header-meta">${timestamp}</div>
  </header>
  <main class="main">
    <div class="summary">
      <div class="stat total"><div class="stat-value">${summary.totalViolations}</div><div class="stat-label">Violations</div></div>
      <div class="stat critical"><div class="stat-value">${summary.criticalCount}</div><div class="stat-label">Critical</div></div>
      <div class="stat serious"><div class="stat-value">${summary.seriousCount}</div><div class="stat-label">Serious</div></div>
      <div class="stat moderate"><div class="stat-value">${summary.moderateCount}</div><div class="stat-label">Moderate</div></div>
      <div class="stat minor"><div class="stat-value">${summary.minorCount}</div><div class="stat-label">Minor</div></div>
    </div>
    ${
      summary.totalViolations === 0
        ? `<div class="all-clear"><div class="all-clear-text">No accessibility violations found across ${summary.totalVariants} variant(s)</div></div>`
        : `<div class="results">${resultItems}</div>`
    }
  </main>
</body>
</html>`;
  }

  /**
   * Generate JSON report for CI integration.
   */
  generateJsonReport(results: A11yResult[]): string {
    const summary = this.getSummary(results);
    return JSON.stringify(
      {
        timestamp: new Date().toISOString(),
        summary,
        results: results.map((r) => ({
          art: path.basename(r.artPath, ".art.vue"),
          variant: r.variantName,
          violations: r.violations,
          passes: r.passes,
          incomplete: r.incomplete,
        })),
      },
      null,
      2,
    );
  }

  /**
   * Get axe-core source code for injection.
   */
  private async getAxeSource(): Promise<string> {
    try {
      const axeCore = await import("axe-core");
      return axeCore.source;
    } catch {
      throw new Error(
        "axe-core is not installed. Install it as a peer dependency: npm install axe-core",
      );
    }
  }

  /**
   * Build axe-core run options from configuration.
   */
  private buildAxeOptions(): Record<string, unknown> {
    const runOnly: Record<string, unknown> = {};

    // Set WCAG level
    const tags: string[] = [];
    switch (this.options.level) {
      case "A":
        tags.push("wcag2a", "wcag21a");
        break;
      case "AA":
        tags.push("wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa");
        break;
      case "AAA":
        tags.push("wcag2a", "wcag2aa", "wcag2aaa", "wcag21a", "wcag21aa", "wcag22aa");
        break;
    }

    if (tags.length > 0) {
      runOnly.type = "tag";
      runOnly.values = tags;
    }

    const rules: Record<string, { enabled: boolean }> = {};

    for (const ruleId of this.options.includeRules) {
      rules[ruleId] = { enabled: true };
    }
    for (const ruleId of this.options.excludeRules) {
      rules[ruleId] = { enabled: false };
    }

    return {
      ...(Object.keys(runOnly).length > 0 ? { runOnly } : {}),
      ...(Object.keys(rules).length > 0 ? { rules } : {}),
    };
  }

  private buildVariantUrl(baseUrl: string, artPath: string, variantName: string): string {
    const encodedPath = encodeURIComponent(artPath);
    const encodedVariant = encodeURIComponent(variantName);
    return `${baseUrl}/__musea__/preview?art=${encodedPath}&variant=${encodedVariant}`;
  }
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#x27;");
}
