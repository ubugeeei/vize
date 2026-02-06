export interface NativeBinding {
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
  generateArtPalette?: (
    source: string,
    artOptions?: { filename?: string },
    paletteOptions?: { infer_options?: boolean; group_by_type?: boolean },
  ) => {
    title: string;
    controls: Array<{
      name: string;
      control: string;
      default_value?: unknown;
      description?: string;
      required: boolean;
      options: Array<{ label: string; value: unknown }>;
      range?: { min: number; max: number; step?: number };
      group?: string;
    }>;
    groups: string[];
    json: string;
    typescript: string;
  };
  generateArtDoc?: (
    source: string,
    artOptions?: { filename?: string },
    docOptions?: {
      include_source?: boolean;
      include_templates?: boolean;
      include_metadata?: boolean;
    },
  ) => {
    markdown: string;
    filename: string;
    title: string;
    category?: string;
    variant_count: number;
  };
  generateArtCatalog?: (
    sources: string[],
    docOptions?: {
      include_source?: boolean;
      include_templates?: boolean;
      include_metadata?: boolean;
    },
  ) => {
    markdown: string;
    filename: string;
    component_count: number;
    categories: string[];
    tags: string[];
  };
  analyzeSfc?: (
    source: string,
    options?: { filename?: string },
  ) => {
    props: Array<{
      name: string;
      type: string;
      required: boolean;
      default_value?: unknown;
    }>;
    emits: string[];
  };
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
}

export interface ArtInfo {
  path: string;
  title: string;
  description?: string;
  component?: string;
  category?: string;
  tags: string[];
  variantCount: number;
}

export interface ServerContext {
  projectRoot: string;
  loadNative: () => NativeBinding;
  scanArtFiles: () => Promise<Map<string, ArtInfo>>;
  resolveTokensPath: () => Promise<string | null>;
}
