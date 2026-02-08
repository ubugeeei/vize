/**
 * Musea MCP Server — Vue.js design system toolkit.
 *
 * Provides AI assistants with tools to:
 * - Analyze Vue SFC components (props, emits)
 * - Browse and search a component registry
 * - Generate documentation, variants, and Storybook stories
 * - Read and format design tokens
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListResourcesRequestSchema,
  ListToolsRequestSchema,
  ReadResourceRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import fs from "node:fs";
import path from "node:path";
import type { ArtInfo, ServerContext } from "./types.js";
import { loadNative } from "./native.js";
import { findArtFiles } from "./scanner.js";
import { toolDefinitions, handleToolCall } from "./tools.js";
import { listResources, readResource } from "./resources.js";

export function createMuseaServer(config: {
  projectRoot: string;
  include?: string[];
  exclude?: string[];
  tokensPath?: string;
}): Server {
  const server = new Server(
    { name: "musea-mcp-server", version: "0.0.1-alpha.11" },
    { capabilities: { resources: {}, tools: {} } },
  );

  const projectRoot = config.projectRoot;
  const include = config.include ?? ["**/*.art.vue"];
  const exclude = config.exclude ?? ["node_modules/**", "dist/**"];
  const tokensPath = config.tokensPath;

  let artCache: Map<string, ArtInfo> = new Map();
  let lastScanTime = 0;

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
          component: parsed.metadata.component,
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

  async function resolveTokensPath(): Promise<string | null> {
    if (tokensPath) return path.resolve(projectRoot, tokensPath);

    const candidates = ["tokens", "design-tokens", "style-dictionary"];
    for (const dir of candidates) {
      const candidate = path.join(projectRoot, dir);
      try {
        const stat = await fs.promises.stat(candidate);
        if (stat.isDirectory() || stat.isFile()) return candidate;
      } catch {
        // not found
      }
    }
    return null;
  }

  const ctx: ServerContext = { projectRoot, loadNative, scanArtFiles, resolveTokensPath };

  server.setRequestHandler(ListResourcesRequestSchema, () => listResources(ctx));
  server.setRequestHandler(ReadResourceRequestSchema, (req) => readResource(ctx, req.params.uri));
  server.setRequestHandler(ListToolsRequestSchema, () =>
    Promise.resolve({ tools: toolDefinitions }),
  );
  server.setRequestHandler(CallToolRequestSchema, (req) =>
    handleToolCall(ctx, req.params.name, req.params.arguments),
  );

  return server;
}

export async function startServer(
  projectRoot: string,
  options?: { tokensPath?: string },
): Promise<void> {
  const server = createMuseaServer({ projectRoot, tokensPath: options?.tokensPath });
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("[musea-mcp] Server started");
}

export default createMuseaServer;
