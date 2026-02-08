import fs from "node:fs";
import path from "node:path";
import { ErrorCode, McpError } from "@modelcontextprotocol/sdk/types.js";
import type { ServerContext } from "./types.js";
import { parseTokensFromPath, generateTokensMarkdown } from "./tokens.js";

// ---------------------------------------------------------------------------
// Tool definitions
// ---------------------------------------------------------------------------

export const toolDefinitions = [
  // --- Component analysis ---------------------------------------------------
  {
    name: "analyze_component",
    description:
      "Statically analyze a Vue SFC to extract its props and emits. Useful for understanding a component's public API when building or reviewing a design system.",
    inputSchema: {
      type: "object" as const,
      properties: {
        path: {
          type: "string",
          description: "Path to the .vue component file (relative to project root)",
        },
      },
      required: ["path"],
    },
  },
  {
    name: "get_palette",
    description:
      "Derive an interactive props palette (control types, defaults, ranges, options) for a component described by an Art file. Helps to understand how props can be tweaked in a design system playground.",
    inputSchema: {
      type: "object" as const,
      properties: {
        path: {
          type: "string",
          description: "Path to the .art.vue file (relative to project root)",
        },
      },
      required: ["path"],
    },
  },

  // --- Component registry ---------------------------------------------------
  {
    name: "list_components",
    description:
      "List components registered in the design system. Returns titles, categories, tags, and variant counts.",
    inputSchema: {
      type: "object" as const,
      properties: {
        category: { type: "string", description: "Filter by category" },
        tag: { type: "string", description: "Filter by tag" },
      },
    },
  },
  {
    name: "get_component",
    description:
      "Get full details of a design-system component: metadata, variant list, and script/style information.",
    inputSchema: {
      type: "object" as const,
      properties: {
        path: {
          type: "string",
          description: "Path to the .art.vue file (relative to project root)",
        },
      },
      required: ["path"],
    },
  },
  {
    name: "get_variant",
    description: "Retrieve a single variant (template and metadata) from a component.",
    inputSchema: {
      type: "object" as const,
      properties: {
        path: { type: "string", description: "Path to the .art.vue file" },
        variant: { type: "string", description: "Variant name" },
      },
      required: ["path", "variant"],
    },
  },
  {
    name: "search_components",
    description: "Full-text search over component titles, descriptions, and tags.",
    inputSchema: {
      type: "object" as const,
      properties: {
        query: { type: "string", description: "Search query" },
      },
      required: ["query"],
    },
  },

  // --- Code generation ------------------------------------------------------
  {
    name: "generate_variants",
    description:
      "Analyze a Vue component's props and auto-generate an .art.vue file containing appropriate variant combinations (default, boolean toggles, enum values, etc.).",
    inputSchema: {
      type: "object" as const,
      properties: {
        componentPath: {
          type: "string",
          description: "Path to the .vue component file (relative to project root)",
        },
        maxVariants: {
          type: "number",
          description: "Maximum number of variants to generate (default: 20)",
        },
        includeDefault: {
          type: "boolean",
          description: "Include a default variant (default: true)",
        },
        includeBooleanToggles: {
          type: "boolean",
          description: "Generate variants that toggle each boolean prop (default: true)",
        },
        includeEnumVariants: {
          type: "boolean",
          description: "Generate one variant per enum/union value (default: true)",
        },
      },
      required: ["componentPath"],
    },
  },
  {
    name: "generate_csf",
    description:
      "Convert an .art.vue file into Storybook CSF 3.0 code for integration with existing Storybook setups.",
    inputSchema: {
      type: "object" as const,
      properties: {
        path: { type: "string", description: "Path to the .art.vue file" },
      },
      required: ["path"],
    },
  },

  // --- Documentation --------------------------------------------------------
  {
    name: "generate_docs",
    description:
      "Generate Markdown documentation for a design-system component from its .art.vue definition.",
    inputSchema: {
      type: "object" as const,
      properties: {
        path: {
          type: "string",
          description: "Path to the .art.vue file (relative to project root)",
        },
        includeSource: {
          type: "boolean",
          description: "Embed source code in the output (default: false)",
        },
        includeTemplates: {
          type: "boolean",
          description: "Embed variant templates in the output (default: false)",
        },
      },
      required: ["path"],
    },
  },
  {
    name: "generate_catalog",
    description:
      "Produce a single Markdown catalog covering every component in the design system, grouped by category.",
    inputSchema: {
      type: "object" as const,
      properties: {
        includeSource: {
          type: "boolean",
          description: "Embed source code in the catalog (default: false)",
        },
        includeTemplates: {
          type: "boolean",
          description: "Embed variant templates in the catalog (default: false)",
        },
      },
    },
  },

  // --- Design tokens --------------------------------------------------------
  {
    name: "get_tokens",
    description:
      "Read design tokens (colors, spacing, typography, etc.) from a Style Dictionary–compatible JSON file or directory. Auto-detects common paths if not specified.",
    inputSchema: {
      type: "object" as const,
      properties: {
        tokensPath: {
          type: "string",
          description:
            "Path to tokens JSON file or directory (relative to project root). Auto-detects tokens/, design-tokens/, or style-dictionary/ if omitted.",
        },
        format: {
          type: "string",
          enum: ["json", "markdown"],
          description: "Output format (default: json)",
        },
      },
    },
  },
];

// ---------------------------------------------------------------------------
// Tool handlers
// ---------------------------------------------------------------------------

type ToolResult = { content: Array<{ type: "text"; text: string }> };

export async function handleToolCall(
  ctx: ServerContext,
  name: string,
  args: Record<string, unknown> | undefined,
): Promise<ToolResult> {
  const binding = ctx.loadNative();

  switch (name) {
    // --- Component analysis -------------------------------------------------

    case "analyze_component": {
      const vuePath = args?.path as string;
      if (!vuePath) throw new McpError(ErrorCode.InvalidParams, "path is required");
      if (!binding.analyzeSfc) {
        throw new McpError(ErrorCode.InternalError, "analyzeSfc not available in native binding");
      }

      const absolutePath = path.resolve(ctx.projectRoot, vuePath);
      const source = await fs.promises.readFile(absolutePath, "utf-8");
      const analysis = binding.analyzeSfc(source, { filename: absolutePath });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(
              {
                props: analysis.props.map((p) => ({
                  name: p.name,
                  type: p.type,
                  required: p.required,
                  defaultValue: p.default_value,
                })),
                emits: analysis.emits,
              },
              null,
              2,
            ),
          },
        ],
      };
    }

    case "get_palette": {
      const artPath = args?.path as string;
      if (!artPath) throw new McpError(ErrorCode.InvalidParams, "path is required");
      if (!binding.generateArtPalette) {
        throw new McpError(
          ErrorCode.InternalError,
          "generateArtPalette not available in native binding",
        );
      }

      const absolutePath = path.resolve(ctx.projectRoot, artPath);
      const source = await fs.promises.readFile(absolutePath, "utf-8");
      const palette = binding.generateArtPalette(source, { filename: absolutePath });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(
              {
                title: palette.title,
                controls: palette.controls.map((c) => ({
                  name: c.name,
                  control: c.control,
                  defaultValue: c.default_value,
                  description: c.description,
                  required: c.required,
                  options: c.options,
                  range: c.range,
                  group: c.group,
                })),
                groups: palette.groups,
                json: palette.json,
                typescript: palette.typescript,
              },
              null,
              2,
            ),
          },
        ],
      };
    }

    // --- Component registry -------------------------------------------------

    case "list_components": {
      const arts = await ctx.scanArtFiles();
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
                path: path.relative(ctx.projectRoot, r.path),
                title: r.title,
                description: r.description,
                component: r.component,
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
      if (!artPath) throw new McpError(ErrorCode.InvalidParams, "path is required");

      const absolutePath = path.resolve(ctx.projectRoot, artPath);
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

      const absolutePath = path.resolve(ctx.projectRoot, artPath);
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

    case "search_components": {
      const query = (args?.query as string)?.toLowerCase();
      if (!query) throw new McpError(ErrorCode.InvalidParams, "query is required");

      const arts = await ctx.scanArtFiles();
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
                path: path.relative(ctx.projectRoot, r.path),
                title: r.title,
                description: r.description,
                component: r.component,
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

    // --- Code generation ----------------------------------------------------

    case "generate_variants": {
      const componentRelPath = args?.componentPath as string;
      if (!componentRelPath) {
        throw new McpError(ErrorCode.InvalidParams, "componentPath is required");
      }
      if (!binding.analyzeSfc) {
        throw new McpError(ErrorCode.InternalError, "analyzeSfc not available in native binding");
      }
      if (!binding.generateVariants) {
        throw new McpError(
          ErrorCode.InternalError,
          "generateVariants not available in native binding",
        );
      }

      const absolutePath = path.resolve(ctx.projectRoot, componentRelPath);
      const source = await fs.promises.readFile(absolutePath, "utf-8");

      const analysis = binding.analyzeSfc(source, { filename: absolutePath });
      const props = analysis.props.map((p) => ({
        name: p.name,
        prop_type: p.type,
        required: p.required,
        default_value: p.default_value,
      }));

      const relPath = `./${path.basename(absolutePath)}`;
      const result = binding.generateVariants(relPath, props, {
        max_variants: args?.maxVariants as number | undefined,
        include_default: args?.includeDefault as boolean | undefined,
        include_boolean_toggles: args?.includeBooleanToggles as boolean | undefined,
        include_enum_variants: args?.includeEnumVariants as boolean | undefined,
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(
              {
                componentName: result.component_name,
                artFileContent: result.art_file_content,
                variants: result.variants.map((v) => ({
                  name: v.name,
                  isDefault: v.is_default,
                  props: v.props,
                  description: v.description,
                })),
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
      if (!artPath) throw new McpError(ErrorCode.InvalidParams, "path is required");

      const absolutePath = path.resolve(ctx.projectRoot, artPath);
      const source = await fs.promises.readFile(absolutePath, "utf-8");
      const csf = binding.artToCsf(source, { filename: absolutePath });

      return { content: [{ type: "text", text: csf.code }] };
    }

    // --- Documentation ------------------------------------------------------

    case "generate_docs": {
      const artPath = args?.path as string;
      if (!artPath) throw new McpError(ErrorCode.InvalidParams, "path is required");
      if (!binding.generateArtDoc) {
        throw new McpError(
          ErrorCode.InternalError,
          "generateArtDoc not available in native binding",
        );
      }

      const absolutePath = path.resolve(ctx.projectRoot, artPath);
      const source = await fs.promises.readFile(absolutePath, "utf-8");
      const doc = binding.generateArtDoc(
        source,
        { filename: absolutePath },
        {
          include_source: args?.includeSource as boolean | undefined,
          include_templates: args?.includeTemplates as boolean | undefined,
          include_metadata: true,
        },
      );

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(
              {
                markdown: doc.markdown,
                title: doc.title,
                category: doc.category,
                variantCount: doc.variant_count,
              },
              null,
              2,
            ),
          },
        ],
      };
    }

    case "generate_catalog": {
      if (!binding.generateArtCatalog) {
        throw new McpError(
          ErrorCode.InternalError,
          "generateArtCatalog not available in native binding",
        );
      }

      const arts = await ctx.scanArtFiles();
      const sources: string[] = [];
      for (const [filePath] of arts) {
        const source = await fs.promises.readFile(filePath, "utf-8");
        sources.push(source);
      }

      const catalog = binding.generateArtCatalog(sources, {
        include_source: args?.includeSource as boolean | undefined,
        include_templates: args?.includeTemplates as boolean | undefined,
        include_metadata: true,
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(
              {
                markdown: catalog.markdown,
                componentCount: catalog.component_count,
                categories: catalog.categories,
                tags: catalog.tags,
              },
              null,
              2,
            ),
          },
        ],
      };
    }

    // --- Design tokens ------------------------------------------------------

    case "get_tokens": {
      const inputPath = args?.tokensPath as string | undefined;
      const format = (args?.format as string) ?? "json";

      let resolvedPath: string | null;
      if (inputPath) {
        resolvedPath = path.resolve(ctx.projectRoot, inputPath);
      } else {
        resolvedPath = await ctx.resolveTokensPath();
      }

      if (!resolvedPath) {
        throw new McpError(
          ErrorCode.InvalidParams,
          "No tokens path provided and none auto-detected. Looked for: tokens/, design-tokens/, style-dictionary/ directories.",
        );
      }

      const categories = await parseTokensFromPath(resolvedPath);

      if (format === "markdown") {
        return { content: [{ type: "text", text: generateTokensMarkdown(categories) }] };
      }

      return {
        content: [{ type: "text", text: JSON.stringify({ categories }, null, 2) }],
      };
    }

    default:
      throw new McpError(ErrorCode.MethodNotFound, `Unknown tool: ${name}`);
  }
}
