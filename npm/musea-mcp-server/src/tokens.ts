import fs from "node:fs";
import path from "node:path";

export interface TokenValue {
  value: string | number;
  type?: string;
  description?: string;
}

export interface TokenCategory {
  name: string;
  tokens: Record<string, TokenValue>;
  subcategories?: TokenCategory[];
}

export async function parseTokensFromPath(tokensPath: string): Promise<TokenCategory[]> {
  const stat = await fs.promises.stat(tokensPath);

  if (stat.isDirectory()) {
    const entries = await fs.promises.readdir(tokensPath, { withFileTypes: true });
    const categories: TokenCategory[] = [];

    for (const entry of entries) {
      if (entry.isFile() && (entry.name.endsWith(".json") || entry.name.endsWith(".tokens.json"))) {
        const filePath = path.join(tokensPath, entry.name);
        const content = await fs.promises.readFile(filePath, "utf-8");
        const tokens = JSON.parse(content);
        const categoryName = path
          .basename(entry.name, path.extname(entry.name))
          .replace(".tokens", "");

        categories.push({
          name: formatCategoryName(categoryName),
          tokens: extractTokenValues(tokens),
          subcategories: extractSubcats(tokens),
        });
      }
    }

    return categories;
  }

  const content = await fs.promises.readFile(tokensPath, "utf-8");
  const tokens = JSON.parse(content);
  return flattenTokenStructure(tokens);
}

export function generateTokensMarkdown(categories: TokenCategory[]): string {
  const renderCategory = (category: TokenCategory, level: number = 2): string => {
    const heading = "#".repeat(level);
    let md = `\n${heading} ${category.name}\n\n`;

    if (Object.keys(category.tokens).length > 0) {
      md += "| Token | Value | Description |\n";
      md += "|-------|-------|-------------|\n";
      for (const [name, token] of Object.entries(category.tokens)) {
        md += `| \`${name}\` | \`${token.value}\` | ${token.description || "-"} |\n`;
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

  let markdown = "# Design Tokens\n";
  for (const category of categories) {
    markdown += renderCategory(category);
  }
  return markdown;
}

function isTokenLeaf(value: unknown): boolean {
  if (typeof value !== "object" || value === null) return false;
  const obj = value as Record<string, unknown>;
  return "value" in obj && (typeof obj.value === "string" || typeof obj.value === "number");
}

function extractTokenValues(obj: Record<string, unknown>): Record<string, TokenValue> {
  const tokens: Record<string, TokenValue> = {};
  for (const [key, value] of Object.entries(obj)) {
    if (isTokenLeaf(value)) {
      const raw = value as Record<string, unknown>;
      tokens[key] = {
        value: raw.value as string | number,
        type: raw.type as string | undefined,
        description: raw.description as string | undefined,
      };
    }
  }
  return tokens;
}

function extractSubcats(obj: Record<string, unknown>): TokenCategory[] | undefined {
  const subcategories: TokenCategory[] = [];
  for (const [key, value] of Object.entries(obj)) {
    if (!isTokenLeaf(value) && typeof value === "object" && value !== null) {
      const tokens = extractTokenValues(value as Record<string, unknown>);
      const nested = extractSubcats(value as Record<string, unknown>);
      if (Object.keys(tokens).length > 0 || (nested && nested.length > 0)) {
        subcategories.push({
          name: formatCategoryName(key),
          tokens,
          subcategories: nested,
        });
      }
    }
  }
  return subcategories.length > 0 ? subcategories : undefined;
}

function flattenTokenStructure(tokens: Record<string, unknown>): TokenCategory[] {
  const categories: TokenCategory[] = [];
  for (const [key, value] of Object.entries(tokens)) {
    if (isTokenLeaf(value)) continue;
    if (typeof value === "object" && value !== null) {
      const categoryTokens = extractTokenValues(value as Record<string, unknown>);
      const subcategories = flattenTokenStructure(value as Record<string, unknown>);
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

function formatCategoryName(name: string): string {
  return name
    .replace(/[-_]/g, " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .split(" ")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
    .join(" ");
}
