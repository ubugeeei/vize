/**
 * Variant auto-generation module.
 * Generates .art.vue files from component prop analysis.
 */

import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";

/**
 * Autogen configuration options.
 */
export interface AutogenOptions {
  /** Maximum number of variants to generate (default: 20) */
  maxVariants?: number;
  /** Include a "Default" variant with all default values (default: true) */
  includeDefault?: boolean;
  /** Include boolean toggle variants (default: true) */
  includeBooleanToggles?: boolean;
  /** Include enum/union variants (default: true) */
  includeEnumVariants?: boolean;
  /** Include boundary value variants for numbers (default: false) */
  includeBoundaryValues?: boolean;
  /** Include empty string variants for optional strings (default: false) */
  includeEmptyStrings?: boolean;
}

/**
 * Prop definition for variant generation.
 */
export interface PropDefinition {
  name: string;
  propType: string;
  required: boolean;
  defaultValue?: unknown;
}

/**
 * Generated variant.
 */
export interface GeneratedVariant {
  name: string;
  isDefault: boolean;
  props: Record<string, unknown>;
  description?: string;
}

/**
 * Autogen output.
 */
export interface AutogenOutput {
  variants: GeneratedVariant[];
  artFileContent: string;
  componentName: string;
}

// Native binding types
interface NativeAutogen {
  generateVariants?: (
    componentPath: string,
    props: Array<{
      name: string;
      prop_type: string;
      required: boolean;
      default_value?: unknown;
    }>,
    config?: {
      max_variants?: number;
      include_default?: boolean;
      include_boolean_toggles?: boolean;
      include_enum_variants?: boolean;
      include_boundary_values?: boolean;
      include_empty_strings?: boolean;
    },
  ) => {
    variants: Array<{
      name: string;
      is_default: boolean;
      props: Record<string, unknown>;
      description?: string;
    }>;
    art_file_content: string;
    component_name: string;
  };
  analyzeSfc?: (
    source: string,
    options?: { filename?: string },
  ) => {
    props: Array<{ name: string; type: string; required: boolean; default_value?: unknown }>;
    emits: string[];
  };
}

let native: NativeAutogen | null = null;

function loadNative(): NativeAutogen {
  if (native) return native;
  const require = createRequire(import.meta.url);
  try {
    native = require("@vizejs/native") as NativeAutogen;
    return native;
  } catch (e) {
    throw new Error(
      `Failed to load @vizejs/native. Make sure it's installed and built:\n${String(e)}`,
    );
  }
}

/**
 * Generate .art.vue file for a component.
 *
 * @param componentPath - Path to the Vue component file
 * @param options - Auto-generation options
 * @returns Generated .art.vue content and metadata
 */
export async function generateArtFile(
  componentPath: string,
  options: AutogenOptions = {},
): Promise<AutogenOutput> {
  const absolutePath = path.resolve(componentPath);
  const source = await fs.promises.readFile(absolutePath, "utf-8");

  const binding = loadNative();

  // Analyze component to extract props
  let props: PropDefinition[];
  if (binding.analyzeSfc) {
    const analysis = binding.analyzeSfc(source, { filename: absolutePath });
    props = analysis.props.map((p) => ({
      name: p.name,
      propType: p.type,
      required: p.required,
      defaultValue: p.default_value,
    }));
  } else {
    // Fallback: simple regex-based prop extraction
    props = extractPropsSimple(source);
  }

  if (props.length === 0) {
    // No props found: generate minimal art file
    const componentName = path.basename(componentPath, ".vue");
    const relPath = `./${path.basename(componentPath)}`;
    return {
      variants: [{ name: "Default", isDefault: true, props: {} }],
      artFileContent: generateMinimalArt(componentName, relPath),
      componentName,
    };
  }

  // Use native variant generation if available
  if (binding.generateVariants) {
    const nativeProps = props.map((p) => ({
      name: p.name,
      prop_type: p.propType,
      required: p.required,
      default_value: p.defaultValue,
    }));

    const relPath = `./${path.basename(componentPath)}`;
    const result = binding.generateVariants(relPath, nativeProps, {
      max_variants: options.maxVariants,
      include_default: options.includeDefault,
      include_boolean_toggles: options.includeBooleanToggles,
      include_enum_variants: options.includeEnumVariants,
      include_boundary_values: options.includeBoundaryValues,
      include_empty_strings: options.includeEmptyStrings,
    });

    return {
      variants: result.variants.map((v) => ({
        name: v.name,
        isDefault: v.is_default,
        props: v.props,
        description: v.description,
      })),
      artFileContent: result.art_file_content,
      componentName: result.component_name,
    };
  }

  // Fallback: JS-based generation
  return generateArtFileJs(componentPath, props, options);
}

/**
 * Write generated .art.vue file to disk.
 */
export async function writeArtFile(
  componentPath: string,
  options: AutogenOptions = {},
  outputPath?: string,
): Promise<string> {
  const output = await generateArtFile(componentPath, options);

  const targetPath = outputPath ?? componentPath.replace(/\.vue$/, ".art.vue");

  await fs.promises.mkdir(path.dirname(targetPath), { recursive: true });
  await fs.promises.writeFile(targetPath, output.artFileContent, "utf-8");

  return targetPath;
}

// Simple prop extraction fallback (when native binding not available)
function extractPropsSimple(source: string): PropDefinition[] {
  const props: PropDefinition[] = [];

  // Match defineProps<{ ... }>() or defineProps({ ... })
  const propsMatch = source.match(/defineProps\s*<\s*\{([^}]*)\}\s*>/s);

  if (propsMatch) {
    const propsBlock = propsMatch[1];
    const propLines = propsBlock.split("\n");

    for (const line of propLines) {
      const propMatch = line.trim().match(/^(\w+)(\?)?:\s*(.+?)\s*;?\s*$/);
      if (propMatch) {
        props.push({
          name: propMatch[1],
          propType: propMatch[3].replace(/,\s*$/, ""),
          required: !propMatch[2],
        });
      }
    }
  }

  return props;
}

// Minimal art file for components with no props
function generateMinimalArt(componentName: string, componentPath: string): string {
  return `<art title="${componentName}" component="${componentPath}">
  <variant name="Default" default>
    <${componentName} />
  </variant>
</art>

<script setup lang="ts">
import ${componentName} from '${componentPath}'
</script>
`;
}

// JS-based variant generation fallback
function generateArtFileJs(
  componentPath: string,
  props: PropDefinition[],
  options: AutogenOptions,
): AutogenOutput {
  const componentName = path.basename(componentPath, ".vue");
  const relPath = `./${path.basename(componentPath)}`;
  const maxVariants = options.maxVariants ?? 20;
  const variants: GeneratedVariant[] = [];

  // Default variant
  if (options.includeDefault !== false) {
    const defaultProps: Record<string, unknown> = {};
    for (const prop of props) {
      if (prop.defaultValue !== undefined) {
        defaultProps[prop.name] = prop.defaultValue;
      }
    }
    variants.push({
      name: "Default",
      isDefault: true,
      props: defaultProps,
      description: `${componentName} with default props`,
    });
  }

  // Enum variants
  if (options.includeEnumVariants !== false) {
    for (const prop of props) {
      const unionValues = parseUnionType(prop.propType);
      for (const val of unionValues) {
        if (variants.length >= maxVariants) break;
        const name =
          typeof val === "string" ? toPascalCase(val) : `${toPascalCase(prop.name)}_${String(val)}`;
        variants.push({
          name,
          isDefault: false,
          props: { [prop.name]: val },
          description: `${prop.name} = ${JSON.stringify(val)}`,
        });
      }
    }
  }

  // Boolean toggle variants
  if (options.includeBooleanToggles !== false) {
    for (const prop of props) {
      if (variants.length >= maxVariants) break;
      if (prop.propType.toLowerCase() === "boolean") {
        const nonDefault = prop.defaultValue === true ? false : true;
        variants.push({
          name: nonDefault ? toPascalCase(prop.name) : `No${toPascalCase(prop.name)}`,
          isDefault: false,
          props: { [prop.name]: nonDefault },
          description: `${prop.name} = ${nonDefault}`,
        });
      }
    }
  }

  // Generate art file content
  let content = `<art title="${componentName}" component="${relPath}">\n`;
  for (const variant of variants) {
    const attrs = variant.isDefault ? `name="${variant.name}" default` : `name="${variant.name}"`;
    content += `  <variant ${attrs}>\n`;

    const propsStr = Object.entries(variant.props)
      .map(([k, v]) => {
        if (typeof v === "string") return `${k}="${v}"`;
        if (typeof v === "boolean" && v) return k;
        if (typeof v === "boolean" && !v) return `:${k}="false"`;
        return `:${k}="${JSON.stringify(v)}"`;
      })
      .join(" ");

    content += `    <${componentName}${propsStr ? " " + propsStr : ""} />\n`;
    content += `  </variant>\n\n`;
  }
  content += `</art>\n\n<script setup lang="ts">\nimport ${componentName} from '${relPath}'\n</script>\n`;

  return {
    variants,
    artFileContent: content,
    componentName,
  };
}

function parseUnionType(typeStr: string): unknown[] {
  const trimmed = typeStr.trim();
  if (!trimmed.includes("|")) return [];

  if (trimmed.includes("'") || trimmed.includes('"')) {
    return trimmed
      .split("|")
      .map((s) => s.trim().replace(/^['"]|['"]$/g, ""))
      .filter((s) => s.length > 0);
  }

  const parts = trimmed.split("|").map((s) => s.trim());
  if (parts.every((p) => !isNaN(Number(p)))) {
    return parts.map(Number);
  }

  return [];
}

function toPascalCase(str: string): string {
  return str
    .split(/[\s\-_]+/)
    .filter(Boolean)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join("");
}
