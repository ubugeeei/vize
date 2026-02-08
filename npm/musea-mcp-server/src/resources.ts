import fs from "node:fs";
import path from "node:path";
import { ErrorCode, McpError } from "@modelcontextprotocol/sdk/types.js";
import type { ServerContext } from "./types.js";
import { parseTokensFromPath } from "./tokens.js";

export async function listResources(ctx: ServerContext) {
  const arts = await ctx.scanArtFiles();
  const resources = [];

  for (const [filePath, info] of arts) {
    const relativePath = path.relative(ctx.projectRoot, filePath);

    resources.push({
      uri: `musea://component/${encodeURIComponent(relativePath)}`,
      name: info.title,
      description:
        info.description || `${info.category || "Component"} — ${info.variantCount} variant(s)`,
      mimeType: "application/json",
    });

    resources.push({
      uri: `musea://docs/${encodeURIComponent(relativePath)}`,
      name: `${info.title} — Documentation`,
      description: `Markdown docs for ${info.title}`,
      mimeType: "text/markdown",
    });
  }

  const resolvedTokensPath = await ctx.resolveTokensPath();
  if (resolvedTokensPath) {
    resources.push({
      uri: "musea://tokens",
      name: "Design Tokens",
      description: "Project design tokens (colors, spacing, typography, …)",
      mimeType: "application/json",
    });
  }

  return { resources };
}

export async function readResource(ctx: ServerContext, uri: string) {
  if (uri.startsWith("musea://component/")) {
    const relativePath = decodeURIComponent(uri.slice("musea://component/".length));
    const absolutePath = path.resolve(ctx.projectRoot, relativePath);

    try {
      const source = await fs.promises.readFile(absolutePath, "utf-8");
      const binding = ctx.loadNative();
      const parsed = binding.parseArt(source, { filename: absolutePath });

      return {
        contents: [
          {
            uri,
            mimeType: "application/json",
            text: JSON.stringify(
              {
                path: relativePath,
                metadata: parsed.metadata,
                variants: parsed.variants.map((v) => ({
                  name: v.name,
                  template: v.template,
                  isDefault: v.is_default,
                  skipVrt: v.skip_vrt,
                })),
                hasScriptSetup: parsed.has_script_setup,
                hasScript: parsed.has_script,
                styleCount: parsed.style_count,
              },
              null,
              2,
            ),
          },
        ],
      };
    } catch (e) {
      throw new McpError(ErrorCode.InternalError, `Failed to read component: ${String(e)}`);
    }
  }

  if (uri.startsWith("musea://docs/")) {
    const relativePath = decodeURIComponent(uri.slice("musea://docs/".length));
    const absolutePath = path.resolve(ctx.projectRoot, relativePath);

    try {
      const source = await fs.promises.readFile(absolutePath, "utf-8");
      const binding = ctx.loadNative();

      if (!binding.generateArtDoc) {
        throw new McpError(
          ErrorCode.InternalError,
          "generateArtDoc not available in native binding",
        );
      }

      const doc = binding.generateArtDoc(source, { filename: absolutePath });

      return {
        contents: [{ uri, mimeType: "text/markdown", text: doc.markdown }],
      };
    } catch (e) {
      if (e instanceof McpError) throw e;
      throw new McpError(ErrorCode.InternalError, `Failed to generate docs: ${String(e)}`);
    }
  }

  if (uri === "musea://tokens") {
    const resolvedTokensPath = await ctx.resolveTokensPath();
    if (!resolvedTokensPath) {
      throw new McpError(ErrorCode.InternalError, "No tokens path configured or auto-detected");
    }

    try {
      const categories = await parseTokensFromPath(resolvedTokensPath);
      return {
        contents: [
          {
            uri,
            mimeType: "application/json",
            text: JSON.stringify({ categories }, null, 2),
          },
        ],
      };
    } catch (e) {
      throw new McpError(ErrorCode.InternalError, `Failed to read tokens: ${String(e)}`);
    }
  }

  throw new McpError(ErrorCode.InvalidRequest, `Unknown resource URI: ${uri}`);
}
