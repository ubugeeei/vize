/**
 * Musea MCP Server - AI-accessible component gallery.
 *
 * This MCP server exposes tools and resources for AI assistants to:
 * - List and search components in the gallery
 * - Get component metadata and variants
 * - Generate Storybook CSF from Art files
 * - Access design tokens
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ErrorCode,
  ListResourcesRequestSchema,
  ListToolsRequestSchema,
  McpError,
  ReadResourceRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";

// Native binding interface
interface NativeBinding {
  parseArt: (
    source: string,
    options?: { filename?: string },
  ) => {
    filename: string;
    metadata: {
      title: string;
      description?: string;
      component?: string;
      category?: string;
      tags: string[];
      status: string;
      order?: number;
    };
    variants: Array<{
      name: string;
      template: string;
      is_default: boolean;
      skip_vrt: boolean;
    }>;
    has_script_setup: boolean;
    has_script: boolean;
    style_count: number;
  };
  artToCsf: (
    source: string,
    options?: { filename?: string },
  ) => {
    code: string;
    filename: string;
  };
}

// Load native binding lazily
let native: NativeBinding | null = null;

function loadNative(): NativeBinding {
  if (native) return native;

  const require = createRequire(import.meta.url);
  try {
    native = require("@vizejs/native") as NativeBinding;
    return native;
  } catch (e) {
    throw new Error(`Failed to load @vizejs/native. Make sure it's installed: ${String(e)}`);
  }
}

// Art file info
interface ArtInfo {
  path: string;
  title: string;
  description?: string;
  category?: string;
  tags: string[];
  variantCount: number;
}

/**
 * Create and configure the MCP server.
 */
export function createMuseaServer(config: {
  projectRoot: string;
  include?: string[];
  exclude?: string[];
}): Server {
  const server = new Server(
    {
      name: "musea-mcp-server",
      version: "0.0.1-alpha.11",
    },
    {
      capabilities: {
        resources: {},
        tools: {},
      },
    },
  );

  const projectRoot = config.projectRoot;
  const include = config.include ?? ["**/*.art.vue"];
  const exclude = config.exclude ?? ["node_modules/**", "dist/**"];

  // Cache for art files
  let artCache: Map<string, ArtInfo> = new Map();
  let lastScanTime = 0;

  // Scan for art files
  async function scanArtFiles(): Promise<Map<string, ArtInfo>> {
    const now = Date.now();
    if (now - lastScanTime < 5000 && artCache.size > 0) {
      return artCache;
    }

    const binding = loadNative();
    const files = await findArtFiles(projectRoot, include, exclude);

    artCache = new Map();

    for (const file of files) {
      try {
        const source = await fs.promises.readFile(file, "utf-8");
        const parsed = binding.parseArt(source, { filename: file });

        artCache.set(file, {
          path: file,
          title: parsed.metadata.title,
          description: parsed.metadata.description,
          category: parsed.metadata.category,
          tags: parsed.metadata.tags,
          variantCount: parsed.variants.length,
        });
      } catch (e) {
        console.error(`Failed to parse ${file}:`, e);
      }
    }

    lastScanTime = now;
    return artCache;
  }

  // List resources handler
  server.setRequestHandler(ListResourcesRequestSchema, async () => {
    const arts = await scanArtFiles();
    const resources = [];

    for (const [filePath, info] of arts) {
      const relativePath = path.relative(projectRoot, filePath);
      resources.push({
        uri: `musea://art/${encodeURIComponent(relativePath)}`,
        name: info.title,
        description:
          info.description || `${info.category || "Component"} with ${info.variantCount} variants`,
        mimeType: "application/json",
      });
    }

    return { resources };
  });

  // Read resource handler
  server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
    const { uri } = request.params;

    if (!uri.startsWith("musea://art/")) {
      throw new McpError(ErrorCode.InvalidRequest, `Unknown resource URI: ${uri}`);
    }

    const relativePath = decodeURIComponent(uri.slice("musea://art/".length));
    const absolutePath = path.resolve(projectRoot, relativePath);

    try {
      const source = await fs.promises.readFile(absolutePath, "utf-8");
      const binding = loadNative();
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
      throw new McpError(ErrorCode.InternalError, `Failed to read art file: ${String(e)}`);
    }
  });

  // List tools handler
  server.setRequestHandler(ListToolsRequestSchema, async () => {
    return {
      tools: [
        {
          name: "list_components",
          description: "List all components (Art files) in the project with their metadata",
          inputSchema: {
            type: "object",
            properties: {
              category: {
                type: "string",
                description: "Filter by category",
              },
              tag: {
                type: "string",
                description: "Filter by tag",
              },
            },
          },
        },
        {
          name: "get_component",
          description: "Get detailed information about a specific component",
          inputSchema: {
            type: "object",
            properties: {
              path: {
                type: "string",
                description: "Path to the Art file (relative to project root)",
              },
            },
            required: ["path"],
          },
        },
        {
          name: "get_variant",
          description: "Get a specific variant from a component",
          inputSchema: {
            type: "object",
            properties: {
              path: {
                type: "string",
                description: "Path to the Art file",
              },
              variant: {
                type: "string",
                description: "Name of the variant",
              },
            },
            required: ["path", "variant"],
          },
        },
        {
          name: "generate_csf",
          description: "Generate Storybook CSF 3.0 code from an Art file",
          inputSchema: {
            type: "object",
            properties: {
              path: {
                type: "string",
                description: "Path to the Art file",
              },
            },
            required: ["path"],
          },
        },
        {
          name: "search_components",
          description: "Search components by title, description, or tags",
          inputSchema: {
            type: "object",
            properties: {
              query: {
                type: "string",
                description: "Search query",
              },
            },
            required: ["query"],
          },
        },
      ],
    };
  });

  // Call tool handler
  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;
    const binding = loadNative();

    switch (name) {
      case "list_components": {
        const arts = await scanArtFiles();
        let results = Array.from(arts.values());

        if (args?.category) {
          results = results.filter(
            (a) => a.category?.toLowerCase() === (args.category as string).toLowerCase(),
          );
        }

        if (args?.tag) {
          results = results.filter((a) =>
            a.tags.some((t) => t.toLowerCase() === (args.tag as string).toLowerCase()),
          );
        }

        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(
                results.map((r) => ({
                  path: path.relative(projectRoot, r.path),
                  title: r.title,
                  description: r.description,
                  category: r.category,
                  tags: r.tags,
                  variantCount: r.variantCount,
                })),
                null,
                2,
              ),
            },
          ],
        };
      }

      case "get_component": {
        const artPath = args?.path as string;
        if (!artPath) {
          throw new McpError(ErrorCode.InvalidParams, "path is required");
        }

        const absolutePath = path.resolve(projectRoot, artPath);
        const source = await fs.promises.readFile(absolutePath, "utf-8");
        const parsed = binding.parseArt(source, { filename: absolutePath });

        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(
                {
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
      }

      case "get_variant": {
        const artPath = args?.path as string;
        const variantName = args?.variant as string;

        if (!artPath || !variantName) {
          throw new McpError(ErrorCode.InvalidParams, "path and variant are required");
        }

        const absolutePath = path.resolve(projectRoot, artPath);
        const source = await fs.promises.readFile(absolutePath, "utf-8");
        const parsed = binding.parseArt(source, { filename: absolutePath });

        const variant = parsed.variants.find(
          (v) => v.name.toLowerCase() === variantName.toLowerCase(),
        );

        if (!variant) {
          throw new McpError(ErrorCode.InvalidParams, `Variant "${variantName}" not found`);
        }

        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(
                {
                  name: variant.name,
                  template: variant.template,
                  isDefault: variant.is_default,
                  skipVrt: variant.skip_vrt,
                },
                null,
                2,
              ),
            },
          ],
        };
      }

      case "generate_csf": {
        const artPath = args?.path as string;
        if (!artPath) {
          throw new McpError(ErrorCode.InvalidParams, "path is required");
        }

        const absolutePath = path.resolve(projectRoot, artPath);
        const source = await fs.promises.readFile(absolutePath, "utf-8");
        const csf = binding.artToCsf(source, { filename: absolutePath });

        return {
          content: [
            {
              type: "text",
              text: csf.code,
            },
          ],
        };
      }

      case "search_components": {
        const query = (args?.query as string)?.toLowerCase();
        if (!query) {
          throw new McpError(ErrorCode.InvalidParams, "query is required");
        }

        const arts = await scanArtFiles();
        const results = Array.from(arts.values()).filter(
          (a) =>
            a.title.toLowerCase().includes(query) ||
            a.description?.toLowerCase().includes(query) ||
            a.tags.some((t) => t.toLowerCase().includes(query)),
        );

        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(
                results.map((r) => ({
                  path: path.relative(projectRoot, r.path),
                  title: r.title,
                  description: r.description,
                  category: r.category,
                  tags: r.tags,
                })),
                null,
                2,
              ),
            },
          ],
        };
      }

      default:
        throw new McpError(ErrorCode.MethodNotFound, `Unknown tool: ${name}`);
    }
  });

  return server;
}

// Utility functions

async function findArtFiles(root: string, include: string[], exclude: string[]): Promise<string[]> {
  const files: string[] = [];

  async function scan(dir: string): Promise<void> {
    const entries = await fs.promises.readdir(dir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      const relative = path.relative(root, fullPath);

      // Check exclude
      let excluded = false;
      for (const pattern of exclude) {
        if (matchGlob(relative, pattern) || matchGlob(entry.name, pattern)) {
          excluded = true;
          break;
        }
      }

      if (excluded) continue;

      if (entry.isDirectory()) {
        await scan(fullPath);
      } else if (entry.isFile() && entry.name.endsWith(".art.vue")) {
        for (const pattern of include) {
          if (matchGlob(relative, pattern)) {
            files.push(fullPath);
            break;
          }
        }
      }
    }
  }

  await scan(root);
  return files;
}

function matchGlob(filepath: string, pattern: string): boolean {
  const regex = pattern
    .replace(/\*\*/g, "{{DOUBLE_STAR}}")
    .replace(/\*/g, "[^/]*")
    .replace(/{{DOUBLE_STAR}}/g, ".*")
    .replace(/\./g, "\\.");

  return new RegExp(`^${regex}$`).test(filepath);
}

/**
 * Start the MCP server with stdio transport.
 */
export async function startServer(projectRoot: string): Promise<void> {
  const server = createMuseaServer({ projectRoot });
  const transport = new StdioServerTransport();

  await server.connect(transport);
  console.error("[musea-mcp] Server started");
}

export default createMuseaServer;
