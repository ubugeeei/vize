/**
 * Style Dictionary integration for Musea.
 * Generates design token documentation from Style Dictionary format.
 */

import fs from "node:fs";
import path from "node:path";

/**
 * Design token value.
 */
export interface DesignToken {
  value: string | number;
  type?: string;
  description?: string;
  attributes?: Record<string, unknown>;
  $tier?: "primitive" | "semantic" | "component";
  $reference?: string;
  $resolvedValue?: string | number;
}

/**
 * Token category (e.g., colors, spacing, typography).
 */
export interface TokenCategory {
  name: string;
  tokens: Record<string, DesignToken>;
  subcategories?: TokenCategory[];
}

/**
 * Style Dictionary output format.
 */
export interface StyleDictionaryOutput {
  categories: TokenCategory[];
  metadata: {
    name: string;
    version?: string;
    generatedAt: string;
  };
}

/**
 * Configuration for Style Dictionary integration.
 */
export interface StyleDictionaryConfig {
  /**
   * Path to tokens JSON/JS file or directory.
   */
  tokensPath: string;

  /**
   * Output format for documentation.
   * @default 'html'
   */
  outputFormat?: "html" | "json" | "markdown";

  /**
   * Output directory for generated documentation.
   * @default '.vize/tokens'
   */
  outputDir?: string;

  /**
   * Custom token transformations.
   */
  transforms?: TokenTransform[];
}

/**
 * Token transformation function.
 */
export type TokenTransform = (token: DesignToken, path: string[]) => DesignToken;

/**
 * Parse Style Dictionary tokens file.
 */
export async function parseTokens(tokensPath: string): Promise<TokenCategory[]> {
  const absolutePath = path.resolve(tokensPath);
  const stat = await fs.promises.stat(absolutePath);

  if (stat.isDirectory()) {
    return parseTokenDirectory(absolutePath);
  }

  const content = await fs.promises.readFile(absolutePath, "utf-8");
  const tokens = JSON.parse(content);
  return flattenTokens(tokens);
}

/**
 * Parse tokens from a directory.
 */
async function parseTokenDirectory(dirPath: string): Promise<TokenCategory[]> {
  const entries = await fs.promises.readdir(dirPath, { withFileTypes: true });
  const categories: TokenCategory[] = [];

  for (const entry of entries) {
    if (entry.isFile() && (entry.name.endsWith(".json") || entry.name.endsWith(".tokens.json"))) {
      const filePath = path.join(dirPath, entry.name);
      const content = await fs.promises.readFile(filePath, "utf-8");
      const tokens = JSON.parse(content);
      const categoryName = path
        .basename(entry.name, path.extname(entry.name))
        .replace(".tokens", "");

      categories.push({
        name: formatCategoryName(categoryName),
        tokens: extractTokens(tokens),
        subcategories: extractSubcategories(tokens),
      });
    }
  }

  return categories;
}

/**
 * Flatten nested token structure into categories.
 */
function flattenTokens(tokens: Record<string, unknown>, prefix: string[] = []): TokenCategory[] {
  const categories: TokenCategory[] = [];

  for (const [key, value] of Object.entries(tokens)) {
    if (isTokenValue(value)) {
      // This is a token leaf node
      continue;
    }

    if (typeof value === "object" && value !== null) {
      const categoryTokens = extractTokens(value as Record<string, unknown>);
      const subcategories = flattenTokens(value as Record<string, unknown>, [...prefix, key]);

      if (Object.keys(categoryTokens).length > 0 || subcategories.length > 0) {
        categories.push({
          name: formatCategoryName(key),
          tokens: categoryTokens,
          subcategories: subcategories.length > 0 ? subcategories : undefined,
        });
      }
    }
  }

  return categories;
}

/**
 * Extract token values from an object.
 */
function extractTokens(obj: Record<string, unknown>): Record<string, DesignToken> {
  const tokens: Record<string, DesignToken> = {};

  for (const [key, value] of Object.entries(obj)) {
    if (isTokenValue(value)) {
      tokens[key] = normalizeToken(value as Record<string, unknown>);
    }
  }

  return tokens;
}

/**
 * Extract subcategories from an object.
 */
function extractSubcategories(obj: Record<string, unknown>): TokenCategory[] | undefined {
  const subcategories: TokenCategory[] = [];

  for (const [key, value] of Object.entries(obj)) {
    if (!isTokenValue(value) && typeof value === "object" && value !== null) {
      const categoryTokens = extractTokens(value as Record<string, unknown>);
      const nested = extractSubcategories(value as Record<string, unknown>);

      if (Object.keys(categoryTokens).length > 0 || (nested && nested.length > 0)) {
        subcategories.push({
          name: formatCategoryName(key),
          tokens: categoryTokens,
          subcategories: nested,
        });
      }
    }
  }

  return subcategories.length > 0 ? subcategories : undefined;
}

/**
 * Check if a value is a token definition.
 */
function isTokenValue(value: unknown): boolean {
  if (typeof value !== "object" || value === null) return false;
  const obj = value as Record<string, unknown>;
  return "value" in obj && (typeof obj.value === "string" || typeof obj.value === "number");
}

/**
 * Normalize token to DesignToken interface.
 */
function normalizeToken(raw: Record<string, unknown>): DesignToken {
  const token: DesignToken = {
    value: raw.value as string | number,
    type: raw.type as string | undefined,
    description: raw.description as string | undefined,
    attributes: raw.attributes as Record<string, unknown> | undefined,
  };
  if (raw.$tier === "primitive" || raw.$tier === "semantic" || raw.$tier === "component") {
    token.$tier = raw.$tier;
  }
  if (typeof raw.$reference === "string") {
    token.$reference = raw.$reference;
  }
  return token;
}

/**
 * Format category name for display.
 */
function formatCategoryName(name: string): string {
  return name
    .replace(/[-_]/g, " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .split(" ")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
    .join(" ");
}

/**
 * Flatten nested categories into a flat map keyed by dot-path.
 */
export function buildTokenMap(
  categories: TokenCategory[],
  prefix: string[] = [],
): Record<string, DesignToken> {
  const map: Record<string, DesignToken> = {};

  for (const cat of categories) {
    const catKey = cat.name
      .toLowerCase()
      .replace(/\s+/g, "-");
    const catPath = [...prefix, catKey];

    for (const [name, token] of Object.entries(cat.tokens)) {
      const dotPath = [...catPath, name].join(".");
      map[dotPath] = token;
    }

    if (cat.subcategories) {
      const subMap = buildTokenMap(cat.subcategories, catPath);
      Object.assign(map, subMap);
    }
  }

  return map;
}

const REFERENCE_PATTERN = /^\{(.+)\}$/;
const MAX_RESOLVE_DEPTH = 10;

/**
 * Resolve references in categories, setting $tier, $reference, and $resolvedValue.
 */
export function resolveReferences(
  categories: TokenCategory[],
  tokenMap: Record<string, DesignToken>,
): void {
  for (const cat of categories) {
    for (const token of Object.values(cat.tokens)) {
      resolveTokenReference(token, tokenMap);
    }
    if (cat.subcategories) {
      resolveReferences(cat.subcategories, tokenMap);
    }
  }
}

function resolveTokenReference(
  token: DesignToken,
  tokenMap: Record<string, DesignToken>,
): void {
  if (typeof token.value === "string") {
    const match = token.value.match(REFERENCE_PATTERN);
    if (match) {
      // Preserve explicit tier (e.g. "component"), default to "semantic"
      if (!token.$tier || token.$tier === "primitive") {
        token.$tier = "semantic";
      }
      token.$reference = match[1];
      token.$resolvedValue = resolveValue(match[1], tokenMap, 0, new Set());
      return;
    }
  }
  token.$tier = token.$tier ?? "primitive";
}

function resolveValue(
  ref: string,
  tokenMap: Record<string, DesignToken>,
  depth: number,
  visited: Set<string>,
): string | number | undefined {
  if (depth >= MAX_RESOLVE_DEPTH || visited.has(ref)) return undefined;
  visited.add(ref);

  const target = tokenMap[ref];
  if (!target) return undefined;

  if (typeof target.value === "string") {
    const match = target.value.match(REFERENCE_PATTERN);
    if (match) {
      return resolveValue(match[1], tokenMap, depth + 1, visited);
    }
  }
  return target.value;
}

/**
 * Read raw JSON token file.
 */
export async function readRawTokenFile(
  tokensPath: string,
): Promise<Record<string, unknown>> {
  const content = await fs.promises.readFile(tokensPath, "utf-8");
  return JSON.parse(content) as Record<string, unknown>;
}

/**
 * Write raw JSON token file atomically (write tmp, rename).
 */
export async function writeRawTokenFile(
  tokensPath: string,
  data: Record<string, unknown>,
): Promise<void> {
  const tmpPath = tokensPath + ".tmp";
  await fs.promises.writeFile(tmpPath, JSON.stringify(data, null, 2) + "\n", "utf-8");
  await fs.promises.rename(tmpPath, tokensPath);
}

/**
 * Set a token at a dot-separated path in the raw JSON structure.
 */
export function setTokenAtPath(
  data: Record<string, unknown>,
  dotPath: string,
  token: Omit<DesignToken, "$resolvedValue">,
): void {
  const parts = dotPath.split(".");
  let current: Record<string, unknown> = data;

  for (let i = 0; i < parts.length - 1; i++) {
    const key = parts[i];
    if (typeof current[key] !== "object" || current[key] === null) {
      current[key] = {};
    }
    current = current[key] as Record<string, unknown>;
  }

  const leafKey = parts[parts.length - 1];
  const raw: Record<string, unknown> = { value: token.value };
  if (token.type) raw.type = token.type;
  if (token.description) raw.description = token.description;
  if (token.$tier) raw.$tier = token.$tier;
  if (token.$reference) raw.$reference = token.$reference;
  if (token.attributes) raw.attributes = token.attributes;
  current[leafKey] = raw;
}

/**
 * Delete a token at a dot-separated path, cleaning empty parents.
 */
export function deleteTokenAtPath(
  data: Record<string, unknown>,
  dotPath: string,
): boolean {
  const parts = dotPath.split(".");
  const parents: Array<{ obj: Record<string, unknown>; key: string }> = [];
  let current: Record<string, unknown> = data;

  for (let i = 0; i < parts.length - 1; i++) {
    const key = parts[i];
    if (typeof current[key] !== "object" || current[key] === null) {
      return false;
    }
    parents.push({ obj: current, key });
    current = current[key] as Record<string, unknown>;
  }

  const leafKey = parts[parts.length - 1];
  if (!(leafKey in current)) return false;
  delete current[leafKey];

  // Clean empty parents
  for (let i = parents.length - 1; i >= 0; i--) {
    const { obj, key } = parents[i];
    const child = obj[key] as Record<string, unknown>;
    if (Object.keys(child).length === 0) {
      delete obj[key];
    } else {
      break;
    }
  }

  return true;
}

/**
 * Validate that a semantic reference points to an existing token and has no cycles.
 */
export function validateSemanticReference(
  tokenMap: Record<string, DesignToken>,
  reference: string,
  selfPath?: string,
): { valid: boolean; error?: string } {
  if (!tokenMap[reference]) {
    return { valid: false, error: `Reference target "${reference}" does not exist` };
  }

  // Check for cycles
  const visited = new Set<string>();
  if (selfPath) visited.add(selfPath);
  let current = reference;
  let depth = 0;

  while (depth < MAX_RESOLVE_DEPTH) {
    if (visited.has(current)) {
      return { valid: false, error: `Circular reference detected at "${current}"` };
    }
    visited.add(current);

    const target = tokenMap[current];
    if (!target) break;

    if (typeof target.value === "string") {
      const match = target.value.match(REFERENCE_PATTERN);
      if (match) {
        current = match[1];
        depth++;
        continue;
      }
    }
    break;
  }

  if (depth >= MAX_RESOLVE_DEPTH) {
    return { valid: false, error: "Reference chain too deep (max 10)" };
  }

  return { valid: true };
}

/**
 * Find all tokens that reference the given path.
 */
export function findDependentTokens(
  tokenMap: Record<string, DesignToken>,
  targetPath: string,
): string[] {
  const dependents: string[] = [];
  for (const [path, token] of Object.entries(tokenMap)) {
    if (typeof token.value === "string") {
      const match = token.value.match(REFERENCE_PATTERN);
      if (match && match[1] === targetPath) {
        dependents.push(path);
      }
    }
  }
  return dependents;
}

/**
 * Generate HTML documentation for tokens.
 */
export function generateTokensHtml(categories: TokenCategory[]): string {
  const renderToken = (name: string, token: DesignToken): string => {
    const isColor =
      typeof token.value === "string" &&
      (token.value.startsWith("#") ||
        token.value.startsWith("rgb") ||
        token.value.startsWith("hsl") ||
        token.type === "color");

    return `
      <div class="token">
        <div class="token-preview">
          ${isColor ? `<div class="color-swatch" style="background: ${token.value}"></div>` : ""}
        </div>
        <div class="token-info">
          <div class="token-name">${name}</div>
          <div class="token-value">${token.value}</div>
          ${token.description ? `<div class="token-description">${token.description}</div>` : ""}
        </div>
      </div>
    `;
  };

  const renderCategory = (category: TokenCategory, level: number = 2): string => {
    const heading = `h${Math.min(level, 6)}`;
    let html = `<${heading}>${category.name}</${heading}>`;
    html += '<div class="tokens-grid">';

    for (const [name, token] of Object.entries(category.tokens)) {
      html += renderToken(name, token);
    }

    html += "</div>";

    if (category.subcategories) {
      for (const sub of category.subcategories) {
        html += renderCategory(sub, level + 1);
      }
    }

    return html;
  };

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Design Tokens - Musea</title>
  <style>
    :root {
      --musea-bg: #0d0d0d;
      --musea-bg-secondary: #1a1815;
      --musea-text: #e6e9f0;
      --musea-text-muted: #7b8494;
      --musea-accent: #a34828;
      --musea-border: #3a3530;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: 'Inter', -apple-system, sans-serif;
      background: var(--musea-bg);
      color: var(--musea-text);
      line-height: 1.6;
      padding: 2rem;
    }
    h1 { margin-bottom: 2rem; color: var(--musea-accent); }
    h2 { margin: 2rem 0 1rem; padding-bottom: 0.5rem; border-bottom: 1px solid var(--musea-border); }
    h3, h4, h5, h6 { margin: 1.5rem 0 0.75rem; }
    .tokens-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
      gap: 1rem;
      margin-bottom: 1.5rem;
    }
    .token {
      background: var(--musea-bg-secondary);
      border: 1px solid var(--musea-border);
      border-radius: 8px;
      padding: 1rem;
      display: flex;
      gap: 1rem;
      align-items: center;
    }
    .token-preview {
      flex-shrink: 0;
      width: 48px;
      height: 48px;
      display: flex;
      align-items: center;
      justify-content: center;
    }
    .color-swatch {
      width: 48px;
      height: 48px;
      border-radius: 8px;
      border: 1px solid var(--musea-border);
    }
    .token-info {
      flex: 1;
      min-width: 0;
    }
    .token-name {
      font-weight: 600;
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.875rem;
    }
    .token-value {
      color: var(--musea-text-muted);
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.75rem;
      word-break: break-all;
    }
    .token-description {
      color: var(--musea-text-muted);
      font-size: 0.75rem;
      margin-top: 0.25rem;
    }
  </style>
</head>
<body>
  <h1>Design Tokens</h1>
  ${categories.map((cat) => renderCategory(cat)).join("")}
</body>
</html>`;
}

/**
 * Generate Markdown documentation for tokens.
 */
export function generateTokensMarkdown(categories: TokenCategory[]): string {
  const renderCategory = (category: TokenCategory, level: number = 2): string => {
    const heading = "#".repeat(level);
    let md = `\n${heading} ${category.name}\n\n`;

    if (Object.keys(category.tokens).length > 0) {
      md += "| Token | Value | Description |\n";
      md += "|-------|-------|-------------|\n";

      for (const [name, token] of Object.entries(category.tokens)) {
        const desc = token.description || "-";
        md += `| \`${name}\` | \`${token.value}\` | ${desc} |\n`;
      }
      md += "\n";
    }

    if (category.subcategories) {
      for (const sub of category.subcategories) {
        md += renderCategory(sub, level + 1);
      }
    }

    return md;
  };

  let markdown = "# Design Tokens\n\n";
  markdown += `> Generated by Musea on ${new Date().toISOString()}\n`;

  for (const category of categories) {
    markdown += renderCategory(category);
  }

  return markdown;
}

/**
 * Style Dictionary plugin for Musea.
 */
export async function processStyleDictionary(
  config: StyleDictionaryConfig,
): Promise<StyleDictionaryOutput> {
  const categories = await parseTokens(config.tokensPath);
  const outputDir = config.outputDir ?? ".vize/tokens";
  const outputFormat = config.outputFormat ?? "html";

  // Ensure output directory exists
  await fs.promises.mkdir(outputDir, { recursive: true });

  // Generate documentation
  let content: string;
  let filename: string;

  switch (outputFormat) {
    case "html":
      content = generateTokensHtml(categories);
      filename = "tokens.html";
      break;
    case "markdown":
      content = generateTokensMarkdown(categories);
      filename = "tokens.md";
      break;
    case "json":
    default:
      content = JSON.stringify({ categories }, null, 2);
      filename = "tokens.json";
  }

  const outputPath = path.join(outputDir, filename);
  await fs.promises.writeFile(outputPath, content, "utf-8");

  console.log(`[musea] Generated token documentation: ${outputPath}`);

  return {
    categories,
    metadata: {
      name: path.basename(config.tokensPath),
      generatedAt: new Date().toISOString(),
    },
  };
}

export default processStyleDictionary;

// --- Token Usage Scanner ---

export interface TokenUsageMatch {
  line: number;
  lineContent: string;
  property: string;
}

export interface TokenUsageEntry {
  artPath: string;
  artTitle: string;
  artCategory?: string;
  matches: TokenUsageMatch[];
}

export type TokenUsageMap = Record<string, TokenUsageEntry[]>;

/**
 * Normalize a token value for comparison.
 * - Lowercase, trim
 * - Leading-zero: `.5rem` → `0.5rem`
 * - Short hex: `#fff` → `#ffffff`
 */
export function normalizeTokenValue(value: string | number): string {
  let v = String(value).trim().toLowerCase();

  // Expand short hex (#abc → #aabbcc, #abcd → #aabbccdd)
  const shortHex = v.match(/^#([0-9a-f])([0-9a-f])([0-9a-f])([0-9a-f])?$/);
  if (shortHex) {
    const [, r, g, b, a] = shortHex;
    v = a
      ? `#${r}${r}${g}${g}${b}${b}${a}${a}`
      : `#${r}${r}${g}${g}${b}${b}`;
  }

  // Add leading zero: `.5rem` → `0.5rem`
  v = v.replace(/(?<![0-9])\.(\d)/g, "0.$1");

  return v;
}

const STYLE_BLOCK_RE = /<style[^>]*>([\s\S]*?)<\/style>/g;
const CSS_PROPERTY_RE = /^\s*([\w-]+)\s*:\s*(.+?)\s*;?\s*$/;

/**
 * Scan art file sources for token value matches in `<style>` blocks.
 */
export function scanTokenUsage(
  artFiles: Map<string, { path: string; metadata: { title: string; category?: string } }>,
  tokenMap: Record<string, DesignToken>,
): TokenUsageMap {
  // Build reverse lookup: normalizedValue → tokenPath[]
  const valueLookup = new Map<string, string[]>();
  for (const [tokenPath, token] of Object.entries(tokenMap)) {
    const rawValue = token.$resolvedValue ?? token.value;
    const normalized = normalizeTokenValue(rawValue);
    if (!normalized) continue;
    const existing = valueLookup.get(normalized);
    if (existing) {
      existing.push(tokenPath);
    } else {
      valueLookup.set(normalized, [tokenPath]);
    }
  }

  const usageMap: TokenUsageMap = {};

  for (const [artPath, artInfo] of artFiles) {
    let source: string;
    try {
      source = fs.readFileSync(artPath, "utf-8");
    } catch {
      continue;
    }

    const allLines = source.split("\n");

    // Find style block line offsets
    const styleRegions: Array<{ startLine: number; content: string }> = [];
    let match: RegExpExecArray | null;
    STYLE_BLOCK_RE.lastIndex = 0;
    while ((match = STYLE_BLOCK_RE.exec(source)) !== null) {
      const beforeMatch = source.slice(0, match.index);
      const startTag = source.slice(match.index, match.index + match[0].indexOf(match[1]));
      const startLine = beforeMatch.split("\n").length + startTag.split("\n").length - 1;
      styleRegions.push({ startLine, content: match[1] });
    }

    // Scan each style block line
    for (const region of styleRegions) {
      const lines = region.content.split("\n");
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const propMatch = line.match(CSS_PROPERTY_RE);
        if (!propMatch) continue;

        const property = propMatch[1];
        const valueStr = propMatch[2];

        // Split on whitespace for multi-value properties (e.g., `border: 1px solid #3b82f6`)
        const valueParts = valueStr.split(/\s+/);
        for (const part of valueParts) {
          const normalizedPart = normalizeTokenValue(part);
          const matchingTokens = valueLookup.get(normalizedPart);
          if (!matchingTokens) continue;

          const lineNumber = region.startLine + i;
          const lineContent = allLines[lineNumber - 1]?.trim() ?? line.trim();

          for (const tokenPath of matchingTokens) {
            if (!usageMap[tokenPath]) {
              usageMap[tokenPath] = [];
            }

            // Find or create entry for this art file
            let entry = usageMap[tokenPath].find((e) => e.artPath === artPath);
            if (!entry) {
              entry = {
                artPath,
                artTitle: artInfo.metadata.title,
                artCategory: artInfo.metadata.category,
                matches: [],
              };
              usageMap[tokenPath].push(entry);
            }

            // Avoid duplicate matches on same line+property
            if (!entry.matches.some((m) => m.line === lineNumber && m.property === property)) {
              entry.matches.push({ line: lineNumber, lineContent, property });
            }
          }
        }
      }
    }
  }

  return usageMap;
}
