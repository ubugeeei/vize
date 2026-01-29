// WASM module loader for vize

export interface CompilerOptions {
  mode?: 'function' | 'module';
  ssr?: boolean;
  scopeId?: string | null;
  filename?: string;
  // Internal mock-only property for vapor mode detection
  outputMode?: 'vdom' | 'vapor';
  // Script extension: 'preserve' keeps TypeScript, 'downcompile' (default) transpiles to JS
  scriptExt?: 'preserve' | 'downcompile';
}

export interface CompileResult {
  code: string;
  preamble: string;
  ast: object;
  map?: object | null;
  helpers: string[];
  templates?: string[];
}

export interface SfcBlock {
  content: string;
  loc: { start: number; end: number };
  lang?: string;
  src?: string;
  attrs: Record<string, string>;
}

export interface SfcScriptBlock extends SfcBlock {
  setup: boolean;
}

export interface SfcStyleBlock extends SfcBlock {
  scoped: boolean;
  module?: string;
}

export interface SfcDescriptor {
  filename: string;
  source: string;
  template?: SfcBlock;
  script?: SfcScriptBlock;
  scriptSetup?: SfcScriptBlock;
  styles: SfcStyleBlock[];
  customBlocks: Array<{ type: string; content: string; attrs: Record<string, string> }>;
}

export interface SfcCompileResult {
  descriptor: SfcDescriptor;
  template?: CompileResult;
  script?: {
    code: string;
    bindings?: object;
  };
  css?: string;
  errors?: string[];
  warnings?: string[];
  bindingMetadata?: object;
}

export interface CssCompileOptions {
  scopeId?: string;
  scoped?: boolean;
  minify?: boolean;
  sourceMap?: boolean;
  filename?: string;
  targets?: {
    chrome?: number;
    firefox?: number;
    safari?: number;
    edge?: number;
    ios?: number;
    android?: number;
  };
}

export interface CssCompileResult {
  code: string;
  map?: string;
  cssVars: string[];
  errors: string[];
  warnings: string[];
}

// Musea types
export interface ArtParseOptions {
  filename?: string;
}

export interface ArtMetadata {
  title: string;
  description?: string;
  component?: string;
  category?: string;
  tags: string[];
  status: 'draft' | 'ready' | 'deprecated';
  order?: number;
}

export interface ArtVariant {
  name: string;
  template: string;
  isDefault: boolean;
  skipVrt: boolean;
  args?: Record<string, unknown>;
}

export interface ArtStyleBlock {
  content: string;
  scoped: boolean;
}

export interface ArtDescriptor {
  filename: string;
  metadata: ArtMetadata;
  variants: ArtVariant[];
  hasScriptSetup: boolean;
  hasScript: boolean;
  styleCount: number;
  styles: ArtStyleBlock[];
}

export interface CsfOutput {
  code: string;
  filename: string;
}

// Patina (Linter) types
export interface LintOptions {
  filename?: string;
  /** Rules to enable (if not set, all rules are enabled) */
  enabledRules?: string[];
  /** Override severity for specific rules */
  severityOverrides?: Record<string, 'error' | 'warning' | 'off'>;
  /** Locale for i18n messages (default: 'en') */
  locale?: 'en' | 'ja' | 'zh';
}

export interface LocaleInfo {
  code: string;
  name: string;
}

export interface LintDiagnostic {
  rule: string;
  severity: 'error' | 'warning';
  message: string;
  location: {
    start: { line: number; column: number; offset: number };
    end: { line: number; column: number; offset: number };
  };
  help?: string;
}

export interface LintResult {
  filename: string;
  errorCount: number;
  warningCount: number;
  diagnostics: LintDiagnostic[];
}

export interface LintRule {
  name: string;
  description: string;
  category: string;
  fixable: boolean;
  defaultSeverity: 'error' | 'warning';
}

// Glyph (Formatter) types
export interface FormatOptions {
  printWidth?: number;
  tabWidth?: number;
  useTabs?: boolean;
  semi?: boolean;
  singleQuote?: boolean;
  bracketSpacing?: boolean;
  bracketSameLine?: boolean;
  singleAttributePerLine?: boolean;
}

export interface FormatResult {
  code: string;
  changed: boolean;
}

// Croquis types
export interface CroquisOptions {
  filename?: string;
}

// Binding source (where it comes from)
export type BindingSource =
  | 'props'
  | 'emits'
  | 'model'
  | 'slots'
  | 'ref'
  | 'reactive'
  | 'computed'
  | 'import'
  | 'local'
  | 'function'
  | 'class'
  | 'templateRef'
  | 'unknown';

// Binding metadata
export interface BindingMetadata {
  fromMacro?: string;
  isExported: boolean;
  isImported: boolean;
  isComponent: boolean;
  isDirective: boolean;
  needsValue: boolean;
  usedInTemplate: boolean;
  usedInScript: boolean;
  scopeDepth: number;
}

export interface BindingDisplay {
  name: string;
  kind: string;
  source: BindingSource;
  metadata: BindingMetadata;
  typeAnnotation?: string;
  start: number;
  end: number;
  isUsed: boolean;
  isMutated: boolean;
  referenceCount: number;
  // Template binding info
  bindable: boolean;  // Can be referenced from template
  usedInTemplate: boolean;  // Actually used in template
  fromScriptSetup: boolean;  // Comes from <script setup>
}

// Scope kind (abbreviated)
export type ScopeKind =
  | 'mod'        // module
  | 'setup'      // scriptSetup
  | 'plain'      // nonScriptSetup
  | 'extern'     // externalModule
  | 'vue'        // vueGlobal
  | 'universal'  // runs on both server and client
  | 'server'     // server only (Node.js)
  | 'client'     // client only (browser)
  | 'function'
  | 'arrowFunction'
  | 'block'
  | 'vFor'
  | 'vSlot'
  | 'class'
  | 'staticBlock'
  | 'catch';

export interface ScopeDisplay {
  id: number;
  parentIds?: number[];  // Multiple parent scopes (e.g., setup can access mod, universal, etc.)
  kind: ScopeKind;
  kindStr: string;
  start: number;
  end: number;
  bindings: string[];
  children: number[];
  depth: number;
}

export interface MacroDisplay {
  name: string;
  start: number;
  end: number;
  type_args?: string;
  args?: string;
  binding?: string;
}

// Type export (hoisted from script setup)
export interface TypeExportDisplay {
  name: string;
  kind: 'type' | 'interface';
  start: number;
  end: number;
  hoisted: boolean;  // true if hoisted from script setup to module level
}

// Invalid export in script setup
export interface InvalidExportDisplay {
  name: string;
  kind: 'const' | 'let' | 'var' | 'function' | 'class' | 'default';
  start: number;
  end: number;
  message: string;
}

export interface PropDisplay {
  name: string;
  type_annotation?: string;
  required: boolean;
  has_default: boolean;
}

export interface EmitDisplay {
  name: string;
  payload_type?: string;
}

// Provide key (string or symbol)
export interface ProvideKey {
  type: 'string' | 'symbol';
  value: string;
}

// Provide entry from Rust analysis
export interface ProvideDisplay {
  key: ProvideKey;
  value: string;
  valueType?: string;
  fromComposable?: string;
  start: number;
  end: number;
}

// Inject pattern
export type InjectPattern = 'simple' | 'objectDestructure' | 'arrayDestructure';

// Inject entry from Rust analysis
export interface InjectDisplay {
  key: ProvideKey;
  localName: string;
  defaultValue?: string;
  expectedType?: string;
  pattern: InjectPattern;
  destructuredProps?: string[];
  fromComposable?: string;
  start: number;
  end: number;
}

export interface CssDisplay {
  selector_count: number;
  unused_selectors: Array<{ text: string; start: number; end: number }>;
  v_bind_count: number;
  is_scoped: boolean;
}

export interface CroquisStats {
  binding_count: number;
  unused_binding_count: number;
  scope_count: number;
  macro_count: number;
  type_export_count: number;
  invalid_export_count: number;
  error_count: number;
  warning_count: number;
}

export interface CroquisDiagnostic {
  severity: 'error' | 'warning' | 'info' | 'hint';
  message: string;
  start: number;
  end: number;
  code?: string;
  related: Array<{ message: string; start: number; end: number }>;
}

export interface Croquis {
  component_name?: string;
  is_setup: boolean;
  bindings: BindingDisplay[];
  scopes: ScopeDisplay[];
  macros: MacroDisplay[];
  props: PropDisplay[];
  emits: EmitDisplay[];
  provides: ProvideDisplay[];
  injects: InjectDisplay[];
  typeExports: TypeExportDisplay[];
  invalidExports: InvalidExportDisplay[];
  css?: CssDisplay;
  diagnostics: CroquisDiagnostic[];
  stats: CroquisStats;
}

export interface CroquisResult {
  croquis: Croquis;
  diagnostics: CroquisDiagnostic[];
  /** VIR (Vize Intermediate Representation) text format */
  vir?: string;
}

// TypeCheck types (Canon)
export interface TypeCheckOptions {
  filename?: string;
  strict?: boolean;
  includeVirtualTs?: boolean;
  checkProps?: boolean;
  checkEmits?: boolean;
  checkTemplateBindings?: boolean;
}

export interface TypeCheckRelatedLocation {
  message: string;
  start: number;
  end: number;
  filename?: string;
}

export interface TypeCheckDiagnostic {
  severity: 'error' | 'warning' | 'info' | 'hint';
  message: string;
  start: number;
  end: number;
  code?: string;
  help?: string;
  related: TypeCheckRelatedLocation[];
}

export interface TypeCheckResult {
  diagnostics: TypeCheckDiagnostic[];
  virtualTs?: string;
  errorCount: number;
  warningCount: number;
  analysisTimeMs?: number;
}

export interface TypeCheckCapability {
  name: string;
  description: string;
  severity: string;
}

export interface TypeCheckCapabilities {
  mode: string;
  description: string;
  checks: TypeCheckCapability[];
  notes: string[];
}

// Cross-file analysis types
export interface CrossFileOptions {
  all?: boolean;
  fallthroughAttrs?: boolean;
  componentEmits?: boolean;
  eventBubbling?: boolean;
  provideInject?: boolean;
  uniqueIds?: boolean;
  serverClientBoundary?: boolean;
  errorSuspenseBoundary?: boolean;
  reactivityTracking?: boolean;
  setupContext?: boolean;
  circularDependencies?: boolean;
  maxImportDepth?: number;
  componentResolution?: boolean;
  propsValidation?: boolean;
}

export interface CrossFileDiagnostic {
  type: string;
  code: string;
  severity: 'error' | 'warning' | 'info' | 'hint';
  message: string;
  file: string;
  offset: number;
  endOffset: number;
  relatedLocations?: Array<{
    file: string;
    offset: number;
    message: string;
  }>;
  suggestion?: string;
}

export interface CrossFileStats {
  filesAnalyzed: number;
  vueComponents: number;
  dependencyEdges: number;
  errorCount: number;
  warningCount: number;
  infoCount: number;
  analysisTimeMs: number;
}

export interface CrossFileResult {
  diagnostics: CrossFileDiagnostic[];
  circularDependencies: string[][];
  stats: CrossFileStats;
  filePaths: string[];
}

export interface CrossFileInput {
  path: string;
  source: string;
}

export interface WasmModule {
  compile: (template: string, options: CompilerOptions) => CompileResult;
  compileVapor: (template: string, options: CompilerOptions) => CompileResult;
  compileCss: (css: string, options: CssCompileOptions) => CssCompileResult;
  parseTemplate: (template: string, options: CompilerOptions) => object;
  parseSfc: (source: string, options: CompilerOptions) => SfcDescriptor;
  compileSfc: (source: string, options: CompilerOptions) => SfcCompileResult;
  // Analysis functions
  analyzeSfc: (source: string, options: CroquisOptions) => CroquisResult;
  analyzeCrossFile: (files: CrossFileInput[], options: CrossFileOptions) => CrossFileResult;
  // Musea functions
  parseArt: (source: string, options: ArtParseOptions) => ArtDescriptor;
  artToCsf: (source: string, options: ArtParseOptions) => CsfOutput;
  // Patina (Linter) functions
  lintTemplate: (source: string, options: LintOptions) => LintResult;
  lintSfc: (source: string, options: LintOptions) => LintResult;
  getLintRules: () => LintRule[];
  getLocales: () => LocaleInfo[];
  // Glyph (Formatter) functions
  formatSfc: (source: string, options: FormatOptions) => FormatResult;
  formatTemplate: (source: string, options: FormatOptions) => FormatResult;
  formatScript: (source: string, options: FormatOptions) => FormatResult;
  // Canon (TypeCheck) functions
  typeCheck: (source: string, options: TypeCheckOptions) => TypeCheckResult;
  getTypeCheckCapabilities: () => TypeCheckCapabilities;
  Compiler: new () => {
    compile: (template: string, options: CompilerOptions) => CompileResult;
    compileVapor: (template: string, options: CompilerOptions) => CompileResult;
    compileCss: (css: string, options: CssCompileOptions) => CssCompileResult;
    parse: (template: string, options: CompilerOptions) => object;
    parseSfc: (source: string, options: CompilerOptions) => SfcDescriptor;
    compileSfc: (source: string, options: CompilerOptions) => SfcCompileResult;
    analyzeSfc: (source: string, options: CroquisOptions) => CroquisResult;
  };
}

let wasmModule: WasmModule | null = null;
let loadPromise: Promise<WasmModule> | null = null;
let usingMock = false;

export async function loadWasm(): Promise<WasmModule> {
  if (wasmModule) {
    return wasmModule;
  }

  if (loadPromise) {
    return loadPromise;
  }

  loadPromise = (async () => {
    try {
      // Try to load the actual WASM module
      // For --target web, we need to explicitly call init() before using exports
      const wasm = await import('./vize_vitrine.js');

      // Initialize WASM - required for --target web
      if (wasm.default) {
        await wasm.default();
      }

      // Get mock module to fill in any missing functions
      const mock = createMockModule();

      // Wrapper to transform WASM analyzeSfc output to expected TypeScript format
      const transformAnalyzeSfc = (source: string, options: CroquisOptions): CroquisResult => {
        if (!wasm.analyzeSfc) {
          return mock.analyzeSfc(source, options);
        }

        try {
          const rawResult = wasm.analyzeSfc(source, options);

          // WASM returns data under 'croquis' key
          const croquis = rawResult.croquis || rawResult;

          // Transform raw WASM scopes to expected ScopeDisplay format
          interface RawWasmScope {
            id: number;
            kind: string;
            kindStr?: string;
            parentIds?: number[];
            start: number;
            end: number;
            bindings: string[];  // WASM returns binding names as string array
            depth?: number;
            isTemplateScope?: boolean;
          }

          const rawScopes: RawWasmScope[] = croquis.scopes || [];

          // Build children map from parentIds
          const childrenMap = new Map<number, number[]>();
          for (const scope of rawScopes) {
            const parentIds = scope.parentIds || [];
            for (const parentId of parentIds) {
              const existing = childrenMap.get(parentId) || [];
              existing.push(scope.id);
              childrenMap.set(parentId, existing);
            }
          }

          // Convert to ScopeDisplay format (depth is already provided by WASM)
          const scopes: ScopeDisplay[] = rawScopes.map(scope => ({
            id: scope.id,
            parentIds: scope.parentIds || [],
            kind: scope.kind as ScopeKind,
            kindStr: scope.kindStr || scope.kind,
            start: scope.start,
            end: scope.end,
            bindings: scope.bindings,  // Already string array
            children: childrenMap.get(scope.id) || [],
            depth: scope.depth || 0,
          }));

          // Transform bindings to match BindingDisplay interface
          const bindings: BindingDisplay[] = (croquis.bindings || []).map((b: { name: string; type: string }, i: number) => ({
            name: b.name,
            kind: b.type,
            source: 'script' as BindingSource,
            metadata: {
              isExported: false,
              isImported: false,
              isComponent: false,
              isDirective: false,
              needsValue: true,
              usedInTemplate: true,
              usedInScript: true,
              scopeDepth: 0,
            },
            typeAnnotation: undefined,
            start: i * 10,
            end: i * 10 + 5,
            isUsed: true,
            isMutated: false,
            referenceCount: 1,
            bindable: true,
            usedInTemplate: true,
            fromScriptSetup: croquis.is_setup || false,
          }));

          // Transform macros from WASM
          const macros: MacroDisplay[] = (croquis.macros || []).map((m: { name: string; kind: string; start: number; end: number; typeArgs?: string }) => ({
            name: m.name,
            start: m.start,
            end: m.end,
            type_args: m.typeArgs,
          }));

          // Transform props from WASM
          const props: PropDisplay[] = (croquis.props || []).map((p: { name: string; required: boolean; hasDefault: boolean }) => ({
            name: p.name,
            required: p.required,
            has_default: p.hasDefault,
          }));

          // Transform emits from WASM
          const emits: EmitDisplay[] = (croquis.emits || []).map((e: { name: string }) => ({
            name: e.name,
          }));

          // Pass through provides and injects from WASM
          const provides: ProvideDisplay[] = croquis.provides || [];
          const injects: InjectDisplay[] = croquis.injects || [];

          // Build CroquisResult in expected format
          const result: CroquisResult = {
            croquis: {
              is_setup: croquis.is_setup || false,
              bindings,
              scopes,
              macros,
              props,
              emits,
              provides,
              injects,
              typeExports: croquis.typeExports || [],
              invalidExports: croquis.invalidExports || [],
              diagnostics: croquis.diagnostics || [],
              stats: croquis.stats || {
                binding_count: bindings.length,
                unused_binding_count: 0,
                scope_count: scopes.length,
                macro_count: macros.length,
                type_export_count: 0,
                invalid_export_count: 0,
                error_count: 0,
                warning_count: 0,
              },
            },
            diagnostics: rawResult.diagnostics || [],
            // VIR (Vize Intermediate Representation) text from WASM
            vir: rawResult.vir || '',
          };

          return result;
        } catch (e) {
          console.warn('WASM analyzeSfc failed, falling back to mock:', e);
          return mock.analyzeSfc(source, options);
        }
      };

      // Merge WASM module with mock fallbacks for missing functions
      wasmModule = {
        compile: wasm.compile || mock.compile,
        compileVapor: wasm.compileVapor || mock.compileVapor,
        compileCss: wasm.compileCss || mock.compileCss,
        parseTemplate: wasm.parseTemplate || mock.parseTemplate,
        parseSfc: wasm.parseSfc || mock.parseSfc,
        compileSfc: wasm.compileSfc || mock.compileSfc,
        // Analysis functions - use WASM croquis analyzer
        // Note: Scope spans from WASM may be 0 (not tracked during analyze_script yet)
        // but macros, props, emits, bindings are properly extracted
        analyzeSfc: transformAnalyzeSfc,
        // Cross-file analysis - use Rust CrossFileAnalyzer
        analyzeCrossFile: wasm.analyzeCrossFile || mock.analyzeCrossFile,
        // Musea functions
        parseArt: wasm.parseArt || mock.parseArt,
        artToCsf: wasm.artToCsf || mock.artToCsf,
        // Patina (Linter) functions - may not be in WASM yet
        lintTemplate: wasm.lintTemplate || mock.lintTemplate,
        lintSfc: wasm.lintSfc || mock.lintSfc,
        getLintRules: wasm.getLintRules || mock.getLintRules,
        getLocales: wasm.getLocales || mock.getLocales,
        // Glyph (Formatter) functions - may not be in WASM yet
        formatSfc: wasm.formatSfc || mock.formatSfc,
        formatTemplate: wasm.formatTemplate || mock.formatTemplate,
        formatScript: wasm.formatScript || mock.formatScript,
        // Canon (TypeCheck) functions
        typeCheck: wasm.typeCheck || mock.typeCheck,
        getTypeCheckCapabilities: wasm.getTypeCheckCapabilities || mock.getTypeCheckCapabilities,
        Compiler: wasm.Compiler || mock.Compiler,
      };
      usingMock = false;
      return wasmModule;
    } catch (e) {
      console.warn('WASM module not found, using mock compiler:', e);
      // Return mock module if WASM is not available
      wasmModule = createMockModule();
      usingMock = true;
      return wasmModule;
    }
  })();

  return loadPromise;
}

// Mock module for development when WASM is not built
function createMockModule(): WasmModule {
  const mockCompile = (template: string, options: CompilerOptions): CompileResult => {
    const isVapor = options.outputMode === 'vapor';
    const hasInterpolation = template.includes('{{');
    const hasElement = template.includes('<');
    const hasVIf = template.includes('v-if');
    const hasVFor = template.includes('v-for');
    const hasVOn = template.includes('@') || template.includes('v-on');

    let code = '';
    const helpers: string[] = [];

    if (isVapor) {
      // Vapor mode output
      code = `// Vapor Mode Output\n`;
      code += `import { template, createTextNode, setText, renderEffect } from 'vue/vapor';\n\n`;
      code += `const t0 = template(\`${template.replace(/\{\{[^}]+\}\}/g, '<!>')}\`);\n\n`;
      code += `export function render(_ctx) {\n`;
      code += `  const n0 = t0();\n`;

      if (hasInterpolation) {
        code += `  const x0 = createTextNode();\n`;
        code += `  renderEffect(() => setText(x0, _ctx.msg));\n`;
        helpers.push('template', 'createTextNode', 'setText', 'renderEffect');
      }

      code += `  return n0;\n`;
      code += `}`;
    } else {
      // VDom mode output
      helpers.push('createElementVNode');
      if (hasInterpolation) helpers.push('toDisplayString');
      if (hasVIf) helpers.push('createCommentVNode', 'Fragment', 'openBlock', 'createElementBlock');
      if (hasVFor) helpers.push('renderList', 'Fragment', 'openBlock', 'createElementBlock');
      if (hasVOn) helpers.push('withModifiers');

      const isModule = options.mode === 'module';

      if (isModule) {
        code = `import { ${helpers.join(', ')} } from "vue"\n\n`;
        code += `export function render(_ctx, _cache, $props, $setup, $data, $options) {\n`;
      } else {
        code = `function render(_ctx, _cache) {\n`;
      }

      code += `  return `;

      if (hasElement) {
        const tagMatch = template.match(/<(\w+)/);
        const tag = tagMatch ? tagMatch[1] : 'div';

        if (hasVIf) {
          code += `(_ctx.show)\n`;
          code += `    ? (_openBlock(), _createElementBlock("${tag}", { key: 0 }))\n`;
          code += `    : _createCommentVNode("v-if", true)`;
        } else if (hasVFor) {
          code += `(_openBlock(true), _createElementBlock(_Fragment, null, _renderList(_ctx.items, (item) => {\n`;
          code += `    return (_openBlock(), _createElementBlock("${tag}", { key: item.id }))\n`;
          code += `  }), 128))`;
        } else if (hasInterpolation) {
          const expr = template.match(/\{\{\s*([^}]+)\s*\}\}/)?.[1] || 'msg';
          code += `_createElementVNode("${tag}", null, _toDisplayString(_ctx.${expr.trim()}), 1 /* TEXT */)`;
        } else {
          code += `_createElementVNode("${tag}", null, null, -1 /* HOISTED */)`;
        }
      } else if (hasInterpolation) {
        const expr = template.match(/\{\{\s*([^}]+)\s*\}\}/)?.[1] || '';
        code += `_toDisplayString(_ctx.${expr.trim()})`;
      } else {
        code += `"${template}"`;
      }

      code += `\n}`;
    }

    // Build AST representation
    const ast = buildMockAst(template);

    return {
      code,
      preamble: options.mode === 'function' ? `const { ${helpers.join(', ')} } = Vue\n` : '',
      ast,
      helpers,
    };
  };

  const mockParseSfc = (source: string, _options: CompilerOptions): SfcDescriptor => {
    const templateMatch = source.match(/<template>([\s\S]*?)<\/template>/);
    const scriptMatch = source.match(/<script>([\s\S]*?)<\/script>/);
    const scriptSetupMatch = source.match(/<script\s+setup>([\s\S]*?)<\/script>/);
    const styleMatches = [...source.matchAll(/<style(\s+scoped)?>([\s\S]*?)<\/style>/g)];

    return {
      filename: 'anonymous.vue',
      source,
      template: templateMatch ? {
        content: templateMatch[1],
        loc: { start: 0, end: 0 },
        attrs: {},
      } : undefined,
      script: scriptMatch ? {
        content: scriptMatch[1],
        loc: { start: 0, end: 0 },
        attrs: {},
        setup: false,
      } : undefined,
      scriptSetup: scriptSetupMatch ? {
        content: scriptSetupMatch[1],
        loc: { start: 0, end: 0 },
        attrs: {},
        setup: true,
      } : undefined,
      styles: styleMatches.map(m => ({
        content: m[2],
        loc: { start: 0, end: 0 },
        attrs: {},
        scoped: !!m[1],
      })),
      customBlocks: [],
    };
  };

  const mockCompileSfc = (source: string, options: CompilerOptions): SfcCompileResult => {
    const descriptor = mockParseSfc(source, options);
    const templateResult = descriptor.template
      ? mockCompile(descriptor.template.content, options)
      : undefined;

    // Generate mock script code and extract bindings
    let scriptCode = '';
    const bindings: Record<string, string> = {};

    if (descriptor.scriptSetup) {
      scriptCode = `// Mock compiled script setup\n`;
      scriptCode += `import { ref, computed } from 'vue'\n\n`;
      scriptCode += `export default {\n`;
      scriptCode += `  setup() {\n`;
      scriptCode += `    ${descriptor.scriptSetup.content.split('\n').filter(l => !l.trim().startsWith('import')).join('\n    ')}\n`;
      scriptCode += `    return { /* bindings */ }\n`;
      scriptCode += `  }\n`;
      scriptCode += `}\n`;
      if (templateResult) {
        scriptCode += `\n${templateResult.code}`;
      }

      // Extract mock bindings from script setup content
      const lines = descriptor.scriptSetup.content.split('\n');
      for (const line of lines) {
        // Match: const x = ref(...), const x = computed(...), or defineProps destructure
        const constMatch = line.match(/const\s+(\w+)\s*=/);
        if (constMatch) {
          const varName = constMatch[1];
          if (line.includes('ref(')) {
            bindings[varName] = 'setup-ref';
          } else if (line.includes('computed(')) {
            bindings[varName] = 'setup-computed';
          } else {
            bindings[varName] = 'setup-const';
          }
        }
      }
    } else if (descriptor.script) {
      scriptCode = descriptor.script.content;
    } else {
      scriptCode = 'export default {}';
    }

    return {
      descriptor,
      template: templateResult,
      script: { code: scriptCode },
      css: descriptor.styles.map(s => s.content).join('\n') || undefined,
      bindingMetadata: Object.keys(bindings).length > 0 ? bindings : undefined,
    };
  };

  const mockCompileCss = (css: string, options: CssCompileOptions): CssCompileResult => {
    let code = css;

    // Apply scoping if requested
    if (options.scoped && options.scopeId) {
      // Simple mock scoping - just append attribute selector
      code = css.replace(/([.#]?\w+)(\s*\{)/g, `$1[${options.scopeId}]$2`);
    }

    // Minify if requested
    if (options.minify) {
      code = code
        .replace(/\s+/g, ' ')
        .replace(/\s*{\s*/g, '{')
        .replace(/\s*}\s*/g, '}')
        .replace(/\s*:\s*/g, ':')
        .replace(/\s*;\s*/g, ';')
        .trim();
    }

    // Extract v-bind() expressions
    const cssVars: string[] = [];
    const vBindRegex = /v-bind\(([^)]+)\)/g;
    let match;
    while ((match = vBindRegex.exec(css)) !== null) {
      const expr = match[1].trim().replace(/['"]/g, '');
      cssVars.push(expr);
    }

    return {
      code,
      cssVars,
      errors: [],
      warnings: [],
    };
  };

  // Mock analyze function - defined inline to avoid hoisting issues
  const inlineMockAnalyzeSfc = (source: string, _options: CroquisOptions): CroquisResult => {
    // Parse the SFC to extract information
    const hasScriptSetup = source.includes('<script setup');
    const hasDefineProps = source.includes('defineProps');
    const hasDefineEmits = source.includes('defineEmits');
    const hasScoped = source.includes('<style scoped');

    const bindings: BindingDisplay[] = [];
    const macros: MacroDisplay[] = [];
    const props: PropDisplay[] = [];
    const emits: EmitDisplay[] = [];
    const typeExports: TypeExportDisplay[] = [];
    const invalidExports: InvalidExportDisplay[] = [];

    // Extract ref bindings
    const refMatches = source.matchAll(/const\s+(\w+)\s*=\s*ref\(/g);
    for (const match of refMatches) {
      bindings.push({
        name: match[1],
        kind: 'SetupRef',
        source: 'ref' as BindingSource,
        metadata: {
          isExported: false,
          isImported: false,
          isComponent: false,
          isDirective: false,
          needsValue: true,
          usedInTemplate: true,
          usedInScript: true,
          scopeDepth: 0,
        },
        start: match.index || 0,
        end: (match.index || 0) + match[0].length,
        isUsed: true,
        isMutated: true,
        referenceCount: 1,
      });
    }

    // Extract computed bindings
    const computedMatches = source.matchAll(/const\s+(\w+)\s*=\s*computed\(/g);
    for (const match of computedMatches) {
      bindings.push({
        name: match[1],
        kind: 'SetupComputed',
        source: 'computed' as BindingSource,
        metadata: {
          isExported: false,
          isImported: false,
          isComponent: false,
          isDirective: false,
          needsValue: true,
          usedInTemplate: usedInTpl,
          usedInScript: true,
          scopeDepth: 0,
        },
        start: match.index || 0,
        end: (match.index || 0) + match[0].length,
        isUsed: true,
        isMutated: false,
        referenceCount: 1,
        bindable: true,
        usedInTemplate: usedInTpl,
        fromScriptSetup: true,
      });
    }

    // Extract function bindings
    const functionMatches = source.matchAll(/function\s+(\w+)\s*\(/g);
    for (const match of functionMatches) {
      const name = match[1];
      const usedInTpl = isUsedInTemplate(name);
      bindings.push({
        name,
        kind: 'SetupConst',
        source: 'function' as BindingSource,
        metadata: {
          isExported: false,
          isImported: false,
          isComponent: false,
          isDirective: false,
          needsValue: false,
          usedInTemplate: usedInTpl,
          usedInScript: true,
          scopeDepth: 0,
        },
        start: match.index || 0,
        end: (match.index || 0) + match[0].length,
        isUsed: true,
        isMutated: false,
        referenceCount: 1,
        bindable: true,
        usedInTemplate: usedInTpl,
        fromScriptSetup: true,
      });
    }

    // Extract defineProps
    if (hasDefineProps) {
      // Match both inline types: defineProps<{...}>() and type references: defineProps<TypeName>()
      const propsMatch = source.match(/defineProps<([^>]+)>\s*\(\s*\)/);
      if (propsMatch) {
        const typeArg = propsMatch[1].trim();
        macros.push({
          name: 'defineProps',
          start: propsMatch.index || 0,
          end: (propsMatch.index || 0) + propsMatch[0].length,
          type_args: typeArg,
        });
        // Extract prop names from inline type (if it's an object type)
        if (typeArg.startsWith('{')) {
          const propNameMatches = typeArg.matchAll(/(\w+)(\?)?:/g);
          for (const propMatch of propNameMatches) {
            props.push({
              name: propMatch[1],
              required: !propMatch[2],
              has_default: false,
            });
          }
        }
      }
    }

    // Extract defineEmits
    if (hasDefineEmits) {
      // Match both inline types: defineEmits<{...}>() and type references: defineEmits<TypeName>()
      const emitsMatch = source.match(/defineEmits<([^>]+)>\s*\(\s*\)/);
      if (emitsMatch) {
        const typeArg = emitsMatch[1].trim();
        macros.push({
          name: 'defineEmits',
          start: emitsMatch.index || 0,
          end: (emitsMatch.index || 0) + emitsMatch[0].length,
          type_args: typeArg,
        });
        // Extract emit names from inline type (if it's an object type)
        if (typeArg.startsWith('{')) {
          const emitNameMatches = typeArg.matchAll(/(\w+):/g);
          for (const emitMatch of emitNameMatches) {
            emits.push({
              name: emitMatch[1],
            });
          }
        }
      }
    }

    // Generate VIR text (formal TOML-like format)
    let vir = '';
    vir += '# VIR v0.1\n';
    vir += '\n';

    // Stats section
    vir += '[stats]\n';
    vir += `bindings = ${bindings.length}\n`;
    vir += `macros = ${macros.length}\n`;

    // Macros section
    if (macros.length > 0) {
      vir += '\n[macros]\n';
      for (const macro of macros) {
        vir += `@${macro.name}`;
        if (macro.type_args) {
          vir += `<{\n${macro.type_args.trim()}\n}>`;
        }
        if (macro.identifier) {
          vir += ` -> ${macro.identifier}`;
        }
        vir += ` # ${macro.start}:${macro.end}\n`;
      }
    }

    // Bindings section
    if (bindings.length > 0) {
      vir += '\n[bindings]\n';
      for (const binding of bindings) {
        const flags: string[] = [];
        if (binding.kind === 'SetupRef') flags.push('ref');
        if (binding.metadata?.isUsed !== false) flags.push('used');
        if (binding.metadata?.isMutated) flags.push('mut');
        vir += `${binding.name}: ${binding.source} @0:0`;
        if (flags.length > 0) {
          vir += ` [${flags.join(', ')}]`;
        }
        vir += '\n';
      }
    }

    // CSS section
    const selectorCount = (source.match(/[.#\w][\w-]*\s*\{/g) || []).length;
    const vBindCount = (source.match(/v-bind\(/g) || []).length;
    if (hasScoped) {
      vir += '\n[css]\n';
      vir += `scoped = true\n`;
      vir += `selectors = ${selectorCount}\n`;
      vir += `v_bind = ${vBindCount}\n`;
    }

    // Props section
    if (props.length > 0) {
      vir += '\n[props]\n';
      for (const prop of props) {
        vir += `${prop.name}: ${prop.type || 'any'}\n`;
      }
    }

    // Emits section
    if (emits.length > 0) {
      vir += '\n[emits]\n';
      for (const emit of emits) {
        vir += `${emit.name}\n`;
      }
    }

    // Generate scopes from source analysis
    const scopes: ScopeDisplay[] = [];
    let scopeId = 0;

    // Helper function to strip comments from code (for accurate parsing)
    const stripComments = (code: string): string => {
      // Remove single-line comments
      let result = code.replace(/\/\/.*$/gm, '');
      // Remove multi-line comments
      result = result.replace(/\/\*[\s\S]*?\*\//g, '');
      return result;
    };

    // Helper function to extract declarations (functions, variables) from script content
    const extractDeclarations = (content: string): string[] => {
      const stripped = stripComments(content);
      const names: string[] = [];
      // Functions
      const funcRegex = /(?:export\s+)?(?:async\s+)?function\s+(\w+)/g;
      let match;
      while ((match = funcRegex.exec(stripped)) !== null) {
        names.push(match[1]);
      }
      // const/let/var declarations
      const varRegex = /(?:export\s+)?(?:const|let|var)\s+(\w+)/g;
      while ((match = varRegex.exec(stripped)) !== null) {
        names.push(match[1]);
      }
      return names;
    };

    // Helper function to extract imports from script content
    type ImportInfo = { name: string, path: string, start: number, end: number };
    const extractImports = (content: string, startOffset: number): { names: string[], externalImports: ImportInfo[] } => {
      const stripped = stripComments(content);
      const names: string[] = [];
      const externalImports: ImportInfo[] = [];
      const importRegex = /import\s+(?:type\s+)?(?:(\w+)|{\s*([^}]+)\s*}|\*\s+as\s+(\w+))?\s*(?:,\s*{\s*([^}]+)\s*})?\s*from\s+['"]([^'"]+)['"]/g;
      let match;
      while ((match = importRegex.exec(stripped)) !== null) {
        const defaultImport = match[1];
        const namedImports = match[2];
        const namespaceImport = match[3];
        const additionalNamed = match[4];
        const modulePath = match[5];

        const importedNames: string[] = [];
        if (defaultImport) importedNames.push(defaultImport);
        if (namespaceImport) importedNames.push(namespaceImport);
        if (namedImports) {
          const parsed = namedImports.split(',').map(n => n.trim().split(/\s+as\s+/).pop()?.trim()).filter(Boolean) as string[];
          importedNames.push(...parsed);
        }
        if (additionalNamed) {
          const parsed = additionalNamed.split(',').map(n => n.trim().split(/\s+as\s+/).pop()?.trim()).filter(Boolean) as string[];
          importedNames.push(...parsed);
        }

        names.push(...importedNames);

        // Check if it's an external module (not relative path or alias)
        const isExternal = !modulePath.startsWith('.') && !modulePath.startsWith('@/');
        if (isExternal) {
          externalImports.push({
            name: importedNames.join(', ') || modulePath,
            path: modulePath,
            start: startOffset + match.index,
            end: startOffset + match.index + match[0].length,
          });
        }
      }
      return { names, externalImports };
    };

    // JS universal globals (available everywhere in both server and client)
    const jsuGlobals = [
      'console', 'Math', 'JSON', 'Date', 'Array', 'Object', 'String', 'Number',
      'Boolean', 'Symbol', 'BigInt', 'Map', 'Set', 'WeakMap', 'WeakSet',
      'Promise', 'Proxy', 'Reflect', 'Error', 'TypeError', 'RangeError',
      'parseInt', 'parseFloat', 'isNaN', 'isFinite', 'encodeURI', 'decodeURI',
      'encodeURIComponent', 'decodeURIComponent', 'undefined', 'NaN', 'Infinity',
    ];

    // JS server-only globals (Node.js)
    const jssGlobals = [
      'process', 'Buffer', '__dirname', '__filename', 'module', 'exports', 'require',
      'global', 'setImmediate', 'clearImmediate',
    ];

    // JS client-only globals (Browser)
    const clientGlobals = [
      'window', 'document', 'navigator', 'location', 'history', 'localStorage',
      'sessionStorage', 'fetch', 'XMLHttpRequest', 'WebSocket', 'Worker',
      'requestAnimationFrame', 'cancelAnimationFrame', 'setTimeout', 'clearTimeout',
      'setInterval', 'clearInterval', 'alert', 'confirm', 'prompt',
    ];

    // Vue globals (template-only)
    const vueGlobals = [
      '$refs', '$emit', '$attrs', '$slots', '$props', '$el', '$options',
      '$data', '$watch', '$nextTick', '$forceUpdate',
    ];

    // Track hoisted items for module scope
    const hoistedBindings: string[] = [];

    // Module scope (root) - bindings will be populated later with hoisted items
    const moduleScope: ScopeDisplay = {
      id: scopeId++,
      kind: 'mod',
      kindStr: 'Mod',
      start: 0,
      end: source.length,
      bindings: [], // Will be populated with hoisted items
      children: [],
      depth: 0,
    };
    scopes.push(moduleScope);

    // Detect non-script-setup block (regular <script> without setup attribute)
    const nonSetupScriptMatch = source.match(/<script(?![^>]*setup)[^>]*>([\s\S]*?)<\/script>/);
    if (nonSetupScriptMatch) {
      const nonSetupStart = source.indexOf(nonSetupScriptMatch[0]);
      const nonSetupEnd = nonSetupStart + nonSetupScriptMatch[0].length;
      const nonSetupContent = nonSetupScriptMatch[1];
      // contentStart is where the actual script content begins (after the opening tag)
      const nonSetupContentStart = nonSetupStart + nonSetupScriptMatch[0].indexOf('>') + 1;

      const { names: importNames, externalImports } = extractImports(nonSetupContent, nonSetupContentStart);
      const declNames = extractDeclarations(nonSetupContent);
      const allPlainBindings = [...new Set([...importNames, ...declNames])];

      // Add plain bindings to hoisted (module scope)
      hoistedBindings.push(...allPlainBindings);

      const nonSetupScope: ScopeDisplay = {
        id: scopeId++,
        parentIds: [0],
        kind: 'plain' as ScopeKind,
        kindStr: 'Plain',
        start: nonSetupStart,
        end: nonSetupEnd,
        bindings: allPlainBindings,
        children: [],
        depth: 1,
      };
      moduleScope.children.push(nonSetupScope.id);
      scopes.push(nonSetupScope);

      // Add external module scopes for imports
      for (const ext of externalImports) {
        const externalScope: ScopeDisplay = {
          id: scopeId++,
          parentIds: [nonSetupScope.id],
          kind: 'extern' as ScopeKind,
          kindStr: `Extern (${ext.path})`,
          start: ext.start,
          end: ext.end,
          bindings: ext.name.split(', ').filter(Boolean),
          children: [],
          depth: 2,
        };
        nonSetupScope.children.push(externalScope.id);
        scopes.push(externalScope);
      }
    }

    // Detect setup scope if script setup exists
    if (hasScriptSetup) {
      const scriptSetupMatch = source.match(/<script[^>]*setup[^>]*>([\s\S]*?)<\/script>/);
      if (scriptSetupMatch) {
        const setupStart = source.indexOf(scriptSetupMatch[0]);
        const setupEnd = setupStart + scriptSetupMatch[0].length;
        const setupContent = scriptSetupMatch[1];
        // Use stripped content for detection to avoid matching commented code
        const strippedSetupContent = stripComments(setupContent);
        // contentStart is where the actual script content begins (after the opening tag)
        const contentStart = setupStart + scriptSetupMatch[0].indexOf('>') + 1;

        // Extract imports from script setup
        const { names: setupImportNames, externalImports: setupExternalImports } = extractImports(setupContent, contentStart);

        // Add setup imports to hoisted (module scope)
        hoistedBindings.push(...setupImportNames);

        // Add export types to hoisted (they are already in typeExports)
        hoistedBindings.push(...typeExports.filter(t => t.hoisted).map(t => t.name));

        // Setup scope contains only directly defined bindings (not imports)
        const setupBindings = bindings.map(b => b.name);

        const setupScope: ScopeDisplay = {
          id: scopeId++,
          parentIds: [0],
          kind: 'setup' as ScopeKind,
          kindStr: 'Setup',
          start: setupStart,
          end: setupEnd,
          bindings: setupBindings, // Only directly defined bindings
          children: [],
          depth: 1,
        };
        moduleScope.children.push(setupScope.id);
        scopes.push(setupScope);

        // Add external module scopes for script setup imports
        for (const ext of setupExternalImports) {
          const externalScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'extmod' as ScopeKind,
            kindStr: `ExtMod (${ext.path})`,
            start: ext.start,
            end: ext.end,
            bindings: ext.name.split(', ').filter(Boolean),
            children: [],
            depth: 2,
          };
          setupScope.children.push(externalScope.id);
          scopes.push(externalScope);
        }

        // Detect function scopes inside setup (use original content for correct positions)
        const functionRegex = /function\s+(\w+)\s*\([^)]*\)\s*\{/g;
        let funcMatch;
        while ((funcMatch = functionRegex.exec(setupContent)) !== null) {
          const funcScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'function',
            kindStr: `Function (${funcMatch[1]})`,
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 50,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(funcScope.id);
          scopes.push(funcScope);
        }

        // Detect arrow function scopes
        const arrowRegex = /const\s+(\w+)\s*=\s*\([^)]*\)\s*=>/g;
        while ((funcMatch = arrowRegex.exec(setupContent)) !== null) {
          const arrowScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: `Arrow (${funcMatch[1]})`,
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 50,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(arrowScope.id);
          scopes.push(arrowScope);
        }

        // Detect watch callbacks
        const watchRegex = /watch\s*\([^,]+,\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = watchRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const watchScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'watch',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(watchScope.id);
          scopes.push(watchScope);
        }

        // Detect watchEffect callbacks
        const watchEffectRegex = /watchEffect\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = watchEffectRegex.exec(setupContent)) !== null) {
          const watchEffectScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'watchEffect',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(watchEffectScope.id);
          scopes.push(watchEffectScope);
        }

        // Detect computed callbacks
        const computedRegex = /(?:const|let)\s+(\w+)\s*=\s*computed\s*\(\s*(?:\([^)]*\)\s*)?=>/g;
        while ((funcMatch = computedRegex.exec(setupContent)) !== null) {
          const computedScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: `computed (${funcMatch[1]})`,
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(computedScope.id);
          scopes.push(computedScope);
        }

        // Detect computed with getter/setter
        const computedGetSetRegex = /(?:const|let)\s+(\w+)\s*=\s*computed\s*\(\s*\{/g;
        while ((funcMatch = computedGetSetRegex.exec(setupContent)) !== null) {
          const computedScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'function',
            kindStr: `computed (${funcMatch[1]}) [get/set]`,
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 50,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(computedScope.id);
          scopes.push(computedScope);
        }

        // Detect lifecycle hooks - Client-only lifecycle hooks for SSR
        const clientOnlyHooks = ['onMounted', 'onUnmounted', 'onBeforeMount', 'onBeforeUnmount', 'onUpdated', 'onBeforeUpdate', 'onActivated', 'onDeactivated'];
        const universalHooks = ['onErrorCaptured', 'onRenderTracked', 'onRenderTriggered'];
        const serverOnlyHooks = ['onServerPrefetch'];

        // Client-only hooks - code inside runs only on client
        for (const hook of clientOnlyHooks) {
          const hookRegex = new RegExp(`${hook}\\s*\\(\\s*(?:async\\s*)?\\(?([^)]*)\\)?\\s*=>`, 'g');
          while ((funcMatch = hookRegex.exec(setupContent)) !== null) {
            const hookScope: ScopeDisplay = {
              id: scopeId++,
              parentIds: [setupScope.id],
              kind: 'client' as ScopeKind,
              kindStr: `ClientOnly (${hook})`,
              start: contentStart + funcMatch.index,
              end: contentStart + funcMatch.index + funcMatch[0].length + 30,
              bindings: [],
              children: [],
              depth: 2,
            };
            setupScope.children.push(hookScope.id);
            scopes.push(hookScope);
          }
        }

        // Universal hooks - code runs on both server and client
        for (const hook of universalHooks) {
          const hookRegex = new RegExp(`${hook}\\s*\\(\\s*(?:async\\s*)?\\(?([^)]*)\\)?\\s*=>`, 'g');
          while ((funcMatch = hookRegex.exec(setupContent)) !== null) {
            const hookScope: ScopeDisplay = {
              id: scopeId++,
              parentIds: [setupScope.id],
              kind: 'universal' as ScopeKind,
              kindStr: `Universal (${hook})`,
              start: contentStart + funcMatch.index,
              end: contentStart + funcMatch.index + funcMatch[0].length + 30,
              bindings: [],
              children: [],
              depth: 2,
            };
            setupScope.children.push(hookScope.id);
            scopes.push(hookScope);
          }
        }

        // Server-only hooks
        for (const hook of serverOnlyHooks) {
          const hookRegex = new RegExp(`${hook}\\s*\\(\\s*(?:async\\s*)?\\(?([^)]*)\\)?\\s*=>`, 'g');
          while ((funcMatch = hookRegex.exec(setupContent)) !== null) {
            const hookScope: ScopeDisplay = {
              id: scopeId++,
              parentIds: [setupScope.id],
              kind: 'function',
              kindStr: `ServerOnly (${hook})`,
              start: contentStart + funcMatch.index,
              end: contentStart + funcMatch.index + funcMatch[0].length + 30,
              bindings: [],
              children: [],
              depth: 2,
            };
            setupScope.children.push(hookScope.id);
            scopes.push(hookScope);
          }
        }

        // Detect provide with factory function
        const provideRegex = /provide\s*\(\s*['"][^'"]+['"]\s*,\s*\(\)\s*=>/g;
        while ((funcMatch = provideRegex.exec(setupContent)) !== null) {
          const provideScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'provide factory',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(provideScope.id);
          scopes.push(provideScope);
        }

        // Detect inject with default factory
        const injectRegex = /inject\s*\(\s*['"][^'"]+['"]\s*,\s*\(\)\s*=>/g;
        while ((funcMatch = injectRegex.exec(setupContent)) !== null) {
          const injectScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'inject default',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(injectScope.id);
          scopes.push(injectScope);
        }

        // Detect try-catch blocks
        const tryCatchRegex = /try\s*\{/g;
        while ((funcMatch = tryCatchRegex.exec(setupContent)) !== null) {
          const tryScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'block',
            kindStr: 'try',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(tryScope.id);
          scopes.push(tryScope);
        }

        const catchRegex = /catch\s*\(\s*(\w+)\s*\)\s*\{/g;
        while ((funcMatch = catchRegex.exec(setupContent)) !== null) {
          const catchScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'block',
            kindStr: 'catch',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [funcMatch[1]],
            children: [],
            depth: 2,
          };
          setupScope.children.push(catchScope.id);
          scopes.push(catchScope);
        }

        // Detect for loops
        const forLoopRegex = /for\s*\(\s*(?:const|let|var)\s+(\w+)/g;
        while ((funcMatch = forLoopRegex.exec(setupContent)) !== null) {
          const forScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'block',
            kindStr: 'for',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [funcMatch[1]],
            children: [],
            depth: 2,
          };
          setupScope.children.push(forScope.id);
          scopes.push(forScope);
        }

        // Detect for...of / for...in loops
        const forOfInRegex = /for\s*\(\s*(?:const|let|var)\s+(\w+)\s+(?:of|in)\s+/g;
        while ((funcMatch = forOfInRegex.exec(setupContent)) !== null) {
          const forOfScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'block',
            kindStr: 'for..of/in',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [funcMatch[1]],
            children: [],
            depth: 2,
          };
          setupScope.children.push(forOfScope.id);
          scopes.push(forOfScope);
        }

        // Detect if blocks with block-scoped variables
        const ifLetRegex = /if\s*\([^)]+\)\s*\{[^}]*(?:const|let)\s+(\w+)/g;
        while ((funcMatch = ifLetRegex.exec(setupContent)) !== null) {
          const ifScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'block',
            kindStr: 'if',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: [funcMatch[1]],
            children: [],
            depth: 2,
          };
          setupScope.children.push(ifScope.id);
          scopes.push(ifScope);
        }

        // Detect Array.forEach callbacks
        const forEachRegex = /\.forEach\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = forEachRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const forEachScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'forEach',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(forEachScope.id);
          scopes.push(forEachScope);
        }

        // Detect Array.map callbacks
        const mapRegex = /\.map\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = mapRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const mapScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'map',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(mapScope.id);
          scopes.push(mapScope);
        }

        // Detect Array.filter callbacks
        const filterRegex = /\.filter\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = filterRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const filterScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'filter',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(filterScope.id);
          scopes.push(filterScope);
        }

        // Detect Array.reduce callbacks
        const reduceRegex = /\.reduce\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = reduceRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const reduceScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'reduce',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(reduceScope.id);
          scopes.push(reduceScope);
        }

        // Detect Array.find/findIndex callbacks
        const findRegex = /\.find(?:Index)?\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = findRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const findScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'find',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(findScope.id);
          scopes.push(findScope);
        }

        // Detect Array.some/every callbacks
        const someEveryRegex = /\.(?:some|every)\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = someEveryRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const someEveryScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'some/every',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(someEveryScope.id);
          scopes.push(someEveryScope);
        }

        // Detect Promise.then callbacks
        const thenRegex = /\.then\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = thenRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const thenScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: '.then',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(thenScope.id);
          scopes.push(thenScope);
        }

        // Detect Promise.catch callbacks
        const promiseCatchRegex = /\.catch\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = promiseCatchRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const promiseCatchScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: '.catch',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(promiseCatchScope.id);
          scopes.push(promiseCatchScope);
        }

        // Detect Promise.finally callbacks
        const finallyRegex = /\.finally\s*\(\s*\(\)\s*=>/g;
        while ((funcMatch = finallyRegex.exec(setupContent)) !== null) {
          const finallyScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: '.finally',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(finallyScope.id);
          scopes.push(finallyScope);
        }

        // Detect setTimeout/setInterval callbacks
        const timerRegex = /set(?:Timeout|Interval)\s*\(\s*\(\)\s*=>/g;
        while ((funcMatch = timerRegex.exec(setupContent)) !== null) {
          const timerScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'timer',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(timerScope.id);
          scopes.push(timerScope);
        }

        // Detect nextTick callbacks
        const nextTickRegex = /nextTick\s*\(\s*\(\)\s*=>/g;
        while ((funcMatch = nextTickRegex.exec(setupContent)) !== null) {
          const nextTickScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'nextTick',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(nextTickScope.id);
          scopes.push(nextTickScope);
        }

        // Detect async IIFE
        const asyncIifeRegex = /\(\s*async\s*\(\)\s*=>\s*\{/g;
        while ((funcMatch = asyncIifeRegex.exec(setupContent)) !== null) {
          const asyncIifeScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'async IIFE',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(asyncIifeScope.id);
          scopes.push(asyncIifeScope);
        }
      }
    }

    // Detect v-for scopes in template
    const templateMatch = source.match(/<template>([\s\S]*?)<\/template>/);
    if (templateMatch) {
      const templateTagStart = source.indexOf(templateMatch[0]);
      // templateContentStart is where the actual template content begins (after the opening tag)
      const templateContentStart = templateTagStart + templateMatch[0].indexOf('>') + 1;
      const vForRegex = /v-for="([^"]+)"/g;
      let vForMatch;
      while ((vForMatch = vForRegex.exec(templateMatch[1])) !== null) {
        const expr = vForMatch[1];
        const inMatch = expr.match(/\(?([^)]+)\)?\s+(?:in|of)\s+/);
        const vForBindings: string[] = [];
        if (inMatch) {
          const aliases = inMatch[1].split(',').map(s => s.trim());
          vForBindings.push(...aliases);
        }

        const vForScope: ScopeDisplay = {
          id: scopeId++,
          parentIds: [0],  // Will add vue_global later
          kind: 'vFor',
          kindStr: `v-for`,
          start: templateContentStart + vForMatch.index,
          end: templateContentStart + vForMatch.index + vForMatch[0].length,
          bindings: vForBindings,
          children: [],
          depth: 1,
        };
        (vForScope as any)._isTemplateScope = true;  // Mark for vue_global parent addition
        moduleScope.children.push(vForScope.id);
        scopes.push(vForScope);
      }

      // Detect v-slot scopes
      const vSlotRegex = /v-slot(?::(\w+))?="([^"]+)"/g;
      let vSlotMatch;
      while ((vSlotMatch = vSlotRegex.exec(templateMatch[1])) !== null) {
        const slotName = vSlotMatch[1] || 'default';
        const slotParams = vSlotMatch[2]?.match(/\{?\s*([^}]+)\s*\}?/)?.[1]?.split(',').map(s => s.trim()) || [];
        const vSlotScope: ScopeDisplay = {
          id: scopeId++,
          parentIds: [0],  // Will add vue_global later
          kind: 'vSlot',
          kindStr: `v-slot:${slotName}`,
          start: templateContentStart + vSlotMatch.index,
          end: templateContentStart + vSlotMatch.index + vSlotMatch[0].length,
          bindings: slotParams,
          children: [],
          depth: 1,
        };
        (vSlotScope as any)._isTemplateScope = true;  // Mark for vue_global parent addition
        moduleScope.children.push(vSlotScope.id);
        scopes.push(vSlotScope);
      }

      // Detect inline event handler scopes
      const eventRegex = /@(\w+)="([^"]+)"/g;
      let eventMatch;
      while ((eventMatch = eventRegex.exec(templateMatch[1])) !== null) {
        const handler = eventMatch[2];
        if (handler.includes('=>') || handler.includes('$event')) {
          const eventScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [0],  // Will add vue_global later
            kind: 'arrowFunction',
            kindStr: `@${eventMatch[1]} handler`,
            start: templateContentStart + eventMatch.index,
            end: templateContentStart + eventMatch.index + eventMatch[0].length,
            bindings: ['$event'],
            children: [],
            depth: 1,
          };
          (eventScope as any)._isTemplateScope = true;  // Mark for vue_global parent addition
          moduleScope.children.push(eventScope.id);
          scopes.push(eventScope);
        }
      }
    }

    // Add global scopes (these are implicit, no parent)
    // Order: ~0 = js_global (universal), ~1 = vue_global, ~2 = mod, ...
    const universalScope: ScopeDisplay = {
      id: scopeId++,
      parentIds: [],
      kind: 'universal' as ScopeKind,
      kindStr: 'JsGlobal',
      start: 0,
      end: 0,
      bindings: jsuGlobals,
      children: [],
      depth: 0,
    };
    scopes.unshift(universalScope);  // ~0

    const vueScope: ScopeDisplay = {
      id: scopeId++,
      parentIds: [],
      kind: 'vue' as ScopeKind,
      kindStr: 'Vue',
      start: 0,
      end: 0,
      bindings: vueGlobals,
      children: [],
      depth: 0,
    };
    scopes.splice(1, 0, vueScope);  // Insert at position 1 -> ~1

    const serverScope: ScopeDisplay = {
      id: scopeId++,
      parentIds: [],
      kind: 'server' as ScopeKind,
      kindStr: 'Server',
      start: 0,
      end: 0,
      bindings: jssGlobals,
      children: [],
      depth: 0,
    };
    scopes.push(serverScope);

    const clientScope: ScopeDisplay = {
      id: scopeId++,
      parentIds: [],
      kind: 'client' as ScopeKind,
      kindStr: 'Client',
      start: 0,
      end: 0,
      bindings: clientGlobals,
      children: [],
      depth: 0,
    };
    scopes.push(clientScope);

    // Add vue_global as parent for template scopes (vFor, vSlot, event handlers)
    for (const scope of scopes) {
      if ((scope as any)._isTemplateScope && scope.parentIds) {
        scope.parentIds.push(vueScope.id);
        delete (scope as any)._isTemplateScope;
      }
    }

    // Populate module scope bindings with hoisted items + jsu globals
    moduleScope.bindings = [...new Set([...hoistedBindings, ...jsuGlobals])];

    // Build scope map for O(1) parent lookup
    const scopeMap = new Map<number, ScopeDisplay>();
    for (const s of scopes) scopeMap.set(s.id, s);

    // Get prefix for scope kind (for index and parent references)
    // - `~` = universal (works on both client and server)
    // - `!` = client only (requires client API: window, document, etc.)
    // - `#` = server private (reserved for future Server Components)
    const getScopePrefix = (kind: string): string => {
      switch (kind) {
        case 'client': return '!';
        case 'server': return '#';
        default: return '~';
      }
    };

    // Assign display IDs per prefix type (separate counters for #, ~, !)
    const prefixCounters: Record<string, number> = { '#': 0, '~': 0, '!': 0 };
    const displayIdMap = new Map<number, string>();  // internal id -> "prefix + displayId"
    for (const scope of scopes) {
      const prefix = getScopePrefix(scope.kind);
      const displayId = prefixCounters[prefix]++;
      displayIdMap.set(scope.id, `${prefix}${displayId}`);
    }

    // Get parent references: $ #1, ~4 (comma-separated with display IDs)
    const getParentRefs = (parentIds: number[]): string => {
      if (!parentIds || parentIds.length === 0) return '';
      const refs = parentIds.map(pid => displayIdMap.get(pid) || `#${pid}`);
      return ` $ ${refs.join(', ')}`;
    };

    // Add scopes to VIR
    if (scopes.length > 0) {
      vir += '\n[scopes]\n';
      for (const scope of scopes) {
        const displayId = displayIdMap.get(scope.id) || `#${scope.id}`;
        vir += `${displayId} ${scope.kindStr.toLowerCase()} @${scope.start}:${scope.end}`;
        if (scope.bindings.length > 0) {
          vir += ` {${scope.bindings.join(', ')}}`;
        }
        if (scope.parentIds && scope.parentIds.length > 0) {
          vir += getParentRefs(scope.parentIds);
        }
        vir += '\n';
      }
    }

    // Update stats with scope count
    vir = vir.replace('[stats]\n', `[stats]\nscopes = ${scopes.length}\n`);

    // Extract provides and injects from source
    const provides: ProvideDisplay[] = [];
    const injects: InjectDisplay[] = [];

    // Extract provide() calls
    const provideMatches = source.matchAll(/provide\s*\(\s*(['"])([^'"]+)\1\s*,\s*([^)]+)\)/g);
    for (const match of provideMatches) {
      provides.push({
        key: { type: 'string', value: match[2] },
        value: match[3].trim(),
        start: match.index || 0,
        end: (match.index || 0) + match[0].length,
      });
    }

    // Extract inject() calls - simple pattern
    const injectMatches = source.matchAll(/(?:const|let)\s+(\w+)\s*=\s*inject\s*[^(]*\(\s*(['"])([^'"]+)\2(?:\s*,\s*([^)]+))?\)/g);
    for (const match of injectMatches) {
      injects.push({
        key: { type: 'string', value: match[3] },
        localName: match[1],
        defaultValue: match[4]?.trim(),
        pattern: 'simple',
        start: match.index || 0,
        end: (match.index || 0) + match[0].length,
      });
    }

    // Extract destructured inject() calls
    const destructuredInjectMatches = source.matchAll(/const\s*\{\s*([^}]+)\s*\}\s*=\s*inject\s*[^(]*\(\s*(['"])([^'"]+)\2(?:\s*,\s*([^)]+))?\)/g);
    for (const match of destructuredInjectMatches) {
      const destructuredProps = match[1].split(',').map(p => p.trim().split(':')[0].trim());
      injects.push({
        key: { type: 'string', value: match[3] },
        localName: `{${destructuredProps.join(', ')}}`,
        defaultValue: match[4]?.trim(),
        pattern: 'objectDestructure',
        destructuredProps,
        start: match.index || 0,
        end: (match.index || 0) + match[0].length,
      });
    }

    const croquis: Croquis = {
      is_setup: hasScriptSetup,
      bindings,
      scopes,
      macros,
      props,
      emits,
      provides,
      injects,
      typeExports,
      invalidExports,
      css: hasScoped ? {
        selector_count: (source.match(/[.#\w][\w-]*\s*\{/g) || []).length,
        unused_selectors: [],
        v_bind_count: (source.match(/v-bind\(/g) || []).length,
        is_scoped: true,
      } : undefined,
      diagnostics: [],
      stats: {
        binding_count: bindings.length,
        unused_binding_count: 0,
        scope_count: scopes.length,
        macro_count: macros.length,
        error_count: 0,
        warning_count: 0,
        type_export_count: typeExports.length,
        invalid_export_count: invalidExports.length,
      },
    };

    return {
      croquis,
      diagnostics: [],
      vir,
    };
  };

  class MockCompiler {
    compile(template: string, options: CompilerOptions): CompileResult {
      return mockCompile(template, options);
    }
    compileVapor(template: string, options: CompilerOptions): CompileResult {
      return mockCompile(template, { ...options, outputMode: 'vapor' });
    }
    compileCss(css: string, options: CssCompileOptions): CssCompileResult {
      return mockCompileCss(css, options);
    }
    parse(template: string, _options: CompilerOptions): object {
      return buildMockAst(template);
    }
    parseSfc(source: string, options: CompilerOptions): SfcDescriptor {
      return mockParseSfc(source, options);
    }
    compileSfc(source: string, options: CompilerOptions): SfcCompileResult {
      return mockCompileSfc(source, options);
    }
    analyzeSfc(source: string, options: CroquisOptions): CroquisResult {
      return inlineMockAnalyzeSfc(source, options);
    }
  }

  const mockParseArt = (source: string, options: ArtParseOptions): ArtDescriptor => {
    // Simple mock parsing for Art files
    const titleMatch = source.match(/title="([^"]+)"/);
    const descMatch = source.match(/description="([^"]+)"/);
    const componentMatch = source.match(/component="([^"]+)"/);
    const categoryMatch = source.match(/category="([^"]+)"/);

    // Extract variants
    const variantRegex = /<variant\s+name="([^"]+)"([^>]*)>([\s\S]*?)<\/variant>/g;
    const variants: ArtVariant[] = [];
    let match;
    while ((match = variantRegex.exec(source)) !== null) {
      const [, name, attrs, template] = match;
      variants.push({
        name,
        template: template.trim(),
        isDefault: attrs.includes('default'),
        skipVrt: attrs.includes('skip-vrt'),
      });
    }

    // Extract style blocks
    const styleRegex = /<style(\s+scoped)?>([\s\S]*?)<\/style>/g;
    const styles: ArtStyleBlock[] = [];
    let styleMatch;
    while ((styleMatch = styleRegex.exec(source)) !== null) {
      styles.push({
        content: styleMatch[2],
        scoped: !!styleMatch[1],
      });
    }

    return {
      filename: options.filename || 'anonymous.art.vue',
      metadata: {
        title: titleMatch?.[1] || 'Untitled',
        description: descMatch?.[1],
        component: componentMatch?.[1],
        category: categoryMatch?.[1],
        tags: [],
        status: 'ready',
      },
      variants,
      hasScriptSetup: source.includes('\x3Cscript setup'),
      hasScript: source.includes('\x3Cscript') && !source.includes('\x3Cscript setup'),
      styleCount: styles.length,
      styles,
    };
  };

  const mockArtToCsf = (source: string, options: ArtParseOptions): CsfOutput => {
    const art = mockParseArt(source, options);
    const componentPath = art.metadata.component || './Component.vue';

    let code = `import type { Meta, StoryObj } from '@storybook/vue3';\n`;
    code += `import Component from '${componentPath}';\n\n`;
    code += `const meta: Meta<typeof Component> = {\n`;
    code += `  title: '${art.metadata.title}',\n`;
    code += `  component: Component,\n`;
    code += `  tags: ['autodocs'],\n`;
    code += `};\n\n`;
    code += `export default meta;\n`;
    code += `type Story = StoryObj<typeof meta>;\n\n`;

    for (const variant of art.variants) {
      const exportName = variant.name.replace(/\s+/g, '');
      code += `export const ${exportName}: Story = {\n`;
      code += `  name: '${variant.name}',\n`;
      code += `  render: (args) => ({\n`;
      code += `    components: { Component },\n`;
      code += `    setup() { return { args }; },\n`;
      code += `    template: \`${variant.template.replace(/`/g, '\\`')}\`,\n`;
      code += `  }),\n`;
      code += `};\n\n`;
    }

    const baseName = (options.filename || 'Component').replace('.art.vue', '');
    return {
      code,
      filename: `${baseName}.stories.ts`,
    };
  };

  // Helper to calculate line/column from character offset
  function getPositionFromOffset(source: string, offset: number): { line: number; column: number } {
    const lines = source.substring(0, offset).split('\n');
    return {
      line: lines.length,
      column: lines[lines.length - 1].length + 1, // 1-indexed column
    };
  }

  // Mock lint functions
  const mockLintTemplate = (source: string, options: LintOptions): LintResult => {
    const allDiagnostics: LintDiagnostic[] = [];
    const filename = options.filename || 'anonymous.vue';

    // Helper to check if a rule is enabled
    const isRuleEnabled = (ruleName: string): boolean => {
      if (!options.enabledRules || options.enabledRules.length === 0) {
        return true; // All rules enabled by default
      }
      return options.enabledRules.includes(ruleName);
    };

    // Helper to get severity for a rule
    const getSeverity = (ruleName: string, defaultSeverity: 'error' | 'warning'): 'error' | 'warning' | 'off' => {
      if (options.severityOverrides && ruleName in options.severityOverrides) {
        return options.severityOverrides[ruleName];
      }
      return defaultSeverity;
    };

    // Simple mock lint rules
    // Check for v-for without :key - find elements with v-for but no :key on same element
    const vForRegex = /<(\w+)[^>]*v-for="[^"]+"/g;
    let vForMatch;
    while ((vForMatch = vForRegex.exec(source)) !== null) {
      const ruleName = 'vue/require-v-for-key';
      if (!isRuleEnabled(ruleName)) continue;
      const severity = getSeverity(ruleName, 'error');
      if (severity === 'off') continue;

      // Check if this element has a :key
      const elementEnd = source.indexOf('>', vForMatch.index);
      const elementStr = source.substring(vForMatch.index, elementEnd + 1);
      if (!elementStr.includes(':key=')) {
        const startPos = getPositionFromOffset(source, vForMatch.index);
        const endOffset = vForMatch.index + vForMatch[0].length;
        const endPos = getPositionFromOffset(source, endOffset);
        allDiagnostics.push({
          rule: ruleName,
          severity,
          message: `Elements in iteration expect to have 'v-bind:key' directives. Element: <${vForMatch[1]}>`,
          location: {
            start: { line: startPos.line, column: startPos.column, offset: vForMatch.index },
            end: { line: endPos.line, column: endPos.column, offset: endOffset },
          },
          help: 'Add a `:key` attribute with a unique identifier for each item',
        });
      }
    }

    // Check for v-if with v-for on same element
    const vIfWithVForRuleName = 'vue/no-use-v-if-with-v-for';
    if (isRuleEnabled(vIfWithVForRuleName)) {
      const vIfWithVForSeverity = getSeverity(vIfWithVForRuleName, 'warning');
      if (vIfWithVForSeverity !== 'off') {
        const vIfWithVForRegex = /<(\w+)[^>]*v-for="[^"]*"[^>]*v-if="[^"]*"/g;
        let vIfWithVFor;
        while ((vIfWithVFor = vIfWithVForRegex.exec(source)) !== null) {
          const startPos = getPositionFromOffset(source, vIfWithVFor.index);
          const endOffset = vIfWithVFor.index + vIfWithVFor[0].length;
          const endPos = getPositionFromOffset(source, endOffset);
          allDiagnostics.push({
            rule: vIfWithVForRuleName,
            severity: vIfWithVForSeverity,
            message: 'Avoid using `v-if` with `v-for` on the same element. Use a computed property to filter the list instead.',
            location: {
              start: { line: startPos.line, column: startPos.column, offset: vIfWithVFor.index },
              end: { line: endPos.line, column: endPos.column, offset: endOffset },
            },
            help: 'Use a computed property to pre-filter the list, e.g., `computed: { activeItems() { return items.filter(i => i.active) } }`',
          });
        }
      }
    }

    // Check for :key on <template> elements
    const templateKeyRuleName = 'vue/no-template-key';
    if (isRuleEnabled(templateKeyRuleName)) {
      const templateKeySeverity = getSeverity(templateKeyRuleName, 'error');
      if (templateKeySeverity !== 'off') {
        const templateKeyRegex = /<template[^>]*:key="[^"]*"/g;
        let templateKey;
        while ((templateKey = templateKeyRegex.exec(source)) !== null) {
          const startPos = getPositionFromOffset(source, templateKey.index);
          const endOffset = templateKey.index + templateKey[0].length;
          const endPos = getPositionFromOffset(source, endOffset);
          allDiagnostics.push({
            rule: templateKeyRuleName,
            severity: templateKeySeverity,
            message: '`<template>` cannot have a `:key` attribute',
            location: {
              start: { line: startPos.line, column: startPos.column, offset: templateKey.index },
              end: { line: endPos.line, column: endPos.column, offset: endOffset },
            },
            help: 'Move the `:key` attribute to a real element inside the template',
          });
        }
      }
    }

    return {
      filename,
      errorCount: allDiagnostics.filter(d => d.severity === 'error').length,
      warningCount: allDiagnostics.filter(d => d.severity === 'warning').length,
      diagnostics: allDiagnostics,
    };
  };

  const mockLintSfc = (source: string, options: LintOptions): LintResult => {
    // Extract template from SFC and lint it
    const templateMatch = source.match(/<template>([\s\S]*?)<\/template>/);
    if (templateMatch) {
      return mockLintTemplate(templateMatch[1], options);
    }
    return {
      filename: options.filename || 'anonymous.vue',
      errorCount: 0,
      warningCount: 0,
      diagnostics: [],
    };
  };

  const mockGetLintRules = (): LintRule[] => {
    return [
      // Essential rules - Error prevention
      {
        name: 'vue/require-v-for-key',
        description: 'Require v-bind:key with v-for directives. Keys help Vue identify which items have changed, been added, or removed for efficient DOM updates.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/valid-v-for',
        description: 'Enforce valid v-for directives. The v-for directive requires a specific syntax: "item in items" or "(item, index) in items".',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/valid-v-if',
        description: 'Enforce valid v-if directives. The directive must have a value that evaluates to a boolean.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/valid-v-else',
        description: 'Enforce valid v-else directives. The v-else directive must be preceded by v-if or v-else-if on a sibling element.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/valid-v-else-if',
        description: 'Enforce valid v-else-if directives. Must be preceded by v-if or v-else-if and must have a value.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/valid-v-model',
        description: 'Enforce valid v-model directives. The v-model directive requires a value that is a valid left-hand side expression.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/valid-v-on',
        description: 'Enforce valid v-on directives. Event handlers must have valid syntax and reference existing methods or expressions.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/valid-v-bind',
        description: 'Enforce valid v-bind directives. Dynamic bindings must have valid JavaScript expressions as values.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/valid-v-slot',
        description: 'Enforce valid v-slot directives. Slot directives must be used on template elements or component elements.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/no-dupe-keys',
        description: 'Disallow duplicate keys in v-for. Duplicate keys can cause rendering bugs and performance issues.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/no-dupe-v-else-if',
        description: 'Disallow duplicate conditions in v-if / v-else-if chains. Duplicate conditions are likely a copy-paste error.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/no-duplicate-attributes',
        description: 'Disallow duplicate attributes on elements. Duplicate attributes override each other unexpectedly.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/no-parsing-error',
        description: 'Disallow parsing errors in <template>. Template syntax errors prevent the component from rendering.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/no-reserved-keys',
        description: 'Disallow overwriting reserved keys like $data, $props, $el, $options, $parent, $root, $children, $slots, $refs, $attrs, $listeners, $watch.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/no-template-key',
        description: 'Disallow key attribute on <template>. The key attribute should be placed on real elements inside the template.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/no-textarea-mustache',
        description: 'Disallow mustache interpolations inside <textarea>. Use v-model instead for two-way binding.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/require-component-is',
        description: 'Require v-bind:is on <component> elements. Dynamic components need the is attribute to specify which component to render.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/require-valid-default-prop',
        description: 'Enforce props with default values to have valid types. Object and Array defaults must be returned from a factory function.',
        category: 'Essential',
        fixable: false,
        defaultSeverity: 'error',
      },

      // Strongly Recommended rules - Improving readability
      {
        name: 'vue/no-use-v-if-with-v-for',
        description: 'Disallow using v-if on the same element as v-for. When used together, v-for has higher priority, causing v-if to run on each iteration. Use computed properties to filter instead.',
        category: 'Strongly Recommended',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/no-unused-vars',
        description: 'Disallow unused variable definitions in v-for directives. Unused iteration variables indicate dead code or incomplete implementation.',
        category: 'Strongly Recommended',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/no-unused-components',
        description: 'Disallow registering components that are not used in the template. Unused component registrations increase bundle size.',
        category: 'Strongly Recommended',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/attribute-hyphenation',
        description: 'Enforce attribute naming style: always use hyphenated names (kebab-case) in templates for props.',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/component-definition-name-casing',
        description: 'Enforce specific casing for component definition names. Consistent naming improves searchability.',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/html-closing-bracket-newline',
        description: 'Require or disallow a line break before tag\'s closing bracket. Improves readability for multi-line elements.',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/html-closing-bracket-spacing',
        description: 'Enforce consistent spacing before tag\'s closing bracket. No space for normal tags, one space for self-closing.',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/html-indent',
        description: 'Enforce consistent indentation in <template>. Default is 2 spaces.',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/html-quotes',
        description: 'Enforce quotes style of HTML attributes. Default is double quotes.',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/html-self-closing',
        description: 'Enforce self-closing style for elements without content.',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/max-attributes-per-line',
        description: 'Enforce the maximum number of attributes per line. Improves readability of complex elements.',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/multiline-html-element-content-newline',
        description: 'Require a line break before and after the contents of a multiline element.',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/mustache-interpolation-spacing',
        description: 'Enforce unified spacing in mustache interpolations. Default is one space: {{ value }}.',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/no-multi-spaces',
        description: 'Disallow multiple spaces between attributes. Use single spaces for consistency.',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/no-spaces-around-equal-signs-in-attribute',
        description: 'Disallow spaces around equal signs in attributes. Write attr="value" not attr = "value".',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/prop-name-casing',
        description: 'Enforce camelCase for prop names in JavaScript and kebab-case in templates.',
        category: 'Strongly Recommended',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/require-default-prop',
        description: 'Require default value for non-required props. Makes component behavior predictable.',
        category: 'Strongly Recommended',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/require-prop-types',
        description: 'Require type definitions in props. Type definitions improve documentation and runtime validation.',
        category: 'Strongly Recommended',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/singleline-html-element-content-newline',
        description: 'Require a line break before and after single-line element contents when the element has attributes.',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/v-bind-style',
        description: 'Enforce v-bind directive style: shorthand (:attr) or longhand (v-bind:attr).',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/v-on-style',
        description: 'Enforce v-on directive style: shorthand (@event) or longhand (v-on:event).',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/v-slot-style',
        description: 'Enforce v-slot directive style: shorthand (#slot) or longhand (v-slot:slot).',
        category: 'Strongly Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },

      // Recommended rules - Minimizing arbitrary choices
      {
        name: 'vue/attributes-order',
        description: 'Enforce order of attributes: DEFINITION, LIST_RENDERING, CONDITIONALS, RENDER_MODIFIERS, GLOBAL, UNIQUE, TWO_WAY_BINDING, OTHER_DIRECTIVES, OTHER_ATTR, EVENTS, CONTENT.',
        category: 'Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/component-tags-order',
        description: 'Enforce order of component top-level elements: <script>, <template>, <style> or <template>, <script>, <style>.',
        category: 'Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/no-lone-template',
        description: 'Disallow unnecessary <template> elements. <template> without directives adds no value.',
        category: 'Recommended',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/no-v-html',
        description: 'Disallow use of v-html to prevent XSS attacks. Use alternatives like sanitization libraries.',
        category: 'Recommended',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/order-in-components',
        description: 'Enforce order of properties in components: name, components, props, data, computed, watch, lifecycle hooks, methods.',
        category: 'Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/this-in-template',
        description: 'Disallow usage of "this" in template. Template expressions have implicit access to component instance.',
        category: 'Recommended',
        fixable: true,
        defaultSeverity: 'warning',
      },

      // Vue 3 specific rules
      {
        name: 'vue/no-deprecated-v-on-native-modifier',
        description: 'Disallow using deprecated .native modifier (Vue 3). Use emits option instead.',
        category: 'Essential (Vue 3)',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/no-deprecated-slot-attribute',
        description: 'Disallow deprecated slot attribute (Vue 3). Use v-slot directive instead.',
        category: 'Essential (Vue 3)',
        fixable: true,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/no-deprecated-slot-scope-attribute',
        description: 'Disallow deprecated slot-scope attribute (Vue 3). Use v-slot directive instead.',
        category: 'Essential (Vue 3)',
        fixable: true,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/require-slots-as-functions',
        description: 'Enforce using $slots as functions in Vue 3. $slots.default() instead of $slots.default.',
        category: 'Essential (Vue 3)',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/no-deprecated-v-bind-sync',
        description: 'Disallow deprecated .sync modifier (Vue 3). Use v-model with argument instead.',
        category: 'Essential (Vue 3)',
        fixable: true,
        defaultSeverity: 'error',
      },

      // Script setup specific rules
      {
        name: 'vue/define-macros-order',
        description: 'Enforce order of defineEmits and defineProps compiler macros in <script setup>.',
        category: 'Script Setup',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/define-props-declaration',
        description: 'Enforce declaration style of defineProps: runtime or type-based.',
        category: 'Script Setup',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/define-emits-declaration',
        description: 'Enforce declaration style of defineEmits: runtime or type-based.',
        category: 'Script Setup',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'vue/no-ref-as-operand',
        description: 'Disallow use of ref objects as operands. Use .value to access reactive value.',
        category: 'Script Setup',
        fixable: true,
        defaultSeverity: 'error',
      },
      {
        name: 'vue/no-setup-props-destructure',
        description: 'Disallow destructuring props in setup (loses reactivity). Use toRefs() instead.',
        category: 'Script Setup',
        fixable: false,
        defaultSeverity: 'warning',
      },

      // Vapor mode rules
      {
        name: 'vapor/no-vue-lifecycle-events',
        description: 'Disallow @vue:xxx per-element lifecycle events in Vapor mode. Vapor components should use lifecycle hooks (onMounted, onUnmounted, etc.) instead.',
        category: 'Vapor',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vapor/no-suspense',
        description: 'Warn about <Suspense> usage in Vapor-only applications. Suspense only works when Vapor components render inside VDOM Suspense.',
        category: 'Vapor',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'vapor/prefer-static-class',
        description: 'Prefer static class over dynamic class binding for better performance in Vapor mode. Use :class only when necessary.',
        category: 'Vapor',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vapor/no-inline-template',
        description: 'Disallow inline-template attribute in Vapor mode. Use slot or component composition instead.',
        category: 'Vapor',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vapor/require-vapor-attribute',
        description: 'Suggest adding the vapor attribute to <script setup> blocks for Vapor mode compilation.',
        category: 'Vapor',
        fixable: true,
        defaultSeverity: 'warning',
      },
      {
        name: 'vapor/no-options-api',
        description: 'Disallow Options API patterns in Vapor components. Vapor only supports Composition API.',
        category: 'Vapor',
        fixable: false,
        defaultSeverity: 'error',
      },
      {
        name: 'vapor/no-get-current-instance',
        description: 'Disallow getCurrentInstance() calls in Vapor components. getCurrentInstance() returns null in Vapor mode.',
        category: 'Vapor',
        fixable: false,
        defaultSeverity: 'error',
      },

      // Accessibility rules
      {
        name: 'a11y/img-alt',
        description: 'Require alt attribute on images. Alt text is essential for screen reader users.',
        category: 'Accessibility',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'a11y/anchor-has-content',
        description: 'Require anchor elements to have accessible content. Empty links are not accessible.',
        category: 'Accessibility',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'a11y/heading-has-content',
        description: 'Require heading elements (h1-h6) to have accessible content.',
        category: 'Accessibility',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'a11y/iframe-has-title',
        description: 'Require iframe elements to have a title attribute for screen readers.',
        category: 'Accessibility',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'a11y/no-distracting-elements',
        description: 'Disallow distracting elements like <marquee> and <blink>.',
        category: 'Accessibility',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'a11y/tabindex-no-positive',
        description: 'Disallow positive tabindex values. Positive values disrupt natural tab order.',
        category: 'Accessibility',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'a11y/click-events-have-key-events',
        description: 'Require keyboard event handlers with click events on non-interactive elements.',
        category: 'Accessibility',
        fixable: false,
        defaultSeverity: 'warning',
      },
      {
        name: 'a11y/form-control-has-label',
        description: 'Require form controls to have associated labels for accessibility.',
        category: 'Accessibility',
        fixable: false,
        defaultSeverity: 'warning',
      },
    ];
  };

  // Mock format functions
  const mockFormatSfc = (source: string, options: FormatOptions): FormatResult => {
    const tabWidth = options.tabWidth || 2;
    const useTabs = options.useTabs || false;
    const indent = useTabs ? '\t' : ' '.repeat(tabWidth);
    const semi = options.semi !== false;
    const singleQuote = options.singleQuote || false;
    const bracketSpacing = options.bracketSpacing !== false;
    const singleAttributePerLine = options.singleAttributePerLine || false;

    let formatted = source.replace(/\r\n/g, '\n');

    // Split into SFC blocks for individual formatting
    const templateMatch = formatted.match(/(<template[^>]*>)([\s\S]*?)(<\/template>)/);
    const scriptMatch = formatted.match(/(<script[^>]*>)([\s\S]*?)(<\/script>)/);
    const styleMatch = formatted.match(/(<style[^>]*>)([\s\S]*?)(<\/style>)/);

    // Format template block
    if (templateMatch) {
      const [fullMatch, openTag, content, closeTag] = templateMatch;
      const formattedTemplate = formatTemplateContent(content, indent, singleAttributePerLine);
      formatted = formatted.replace(fullMatch, `${openTag}\n${formattedTemplate}\n${closeTag}`);
    }

    // Format script block
    if (scriptMatch) {
      const [fullMatch, openTag, content, closeTag] = scriptMatch;
      const formattedScript = formatScriptContent(content, indent, semi, singleQuote, bracketSpacing);
      formatted = formatted.replace(fullMatch, `${openTag}\n${formattedScript}\n${closeTag}`);
    }

    // Format style block
    if (styleMatch) {
      const [fullMatch, openTag, content, closeTag] = styleMatch;
      const formattedStyle = formatStyleContent(content, indent);
      formatted = formatted.replace(fullMatch, `${openTag}\n${formattedStyle}\n${closeTag}`);
    }

    // Ensure consistent newlines between blocks
    formatted = formatted.replace(/(<\/(?:template|script|style)>)\n*(<(?:template|script|style))/g, '$1\n\n$2');

    return {
      code: formatted.trim() + '\n',
      changed: formatted.trim() + '\n' !== source,
    };
  };

  // Helper to format template content
  function formatTemplateContent(content: string, indent: string, singleAttributePerLine: boolean): string {
    const lines = content.split('\n');
    const result: string[] = [];
    let depth = 0;

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) continue;

      // Decrease indent for closing tags
      if (trimmed.startsWith('</') || trimmed.startsWith('/>')) {
        depth = Math.max(0, depth - 1);
      }

      // Handle self-closing decrements
      if (trimmed.match(/^<[^/][^>]*\/>$/)) {
        result.push(indent.repeat(depth) + trimmed);
        continue;
      }

      // Handle multiple attributes per line when singleAttributePerLine is true
      if (singleAttributePerLine && trimmed.match(/<\w+\s+[^>]*>/)) {
        const tagMatch = trimmed.match(/^(<\w+)(\s+[^>]+)(\/?>)$/);
        if (tagMatch) {
          const [, tagStart, attrs, tagEnd] = tagMatch;
          const attrList = attrs.trim().split(/\s+(?=[@:a-zA-Z])/);
          if (attrList.length > 1) {
            result.push(indent.repeat(depth) + tagStart);
            for (const attr of attrList) {
              result.push(indent.repeat(depth + 1) + attr.trim());
            }
            result.push(indent.repeat(depth) + tagEnd);
            if (!trimmed.endsWith('/>') && !trimmed.startsWith('</')) {
              depth++;
            }
            continue;
          }
        }
      }

      result.push(indent.repeat(depth) + trimmed);

      // Increase indent after opening tags (not self-closing)
      if (trimmed.match(/^<[^/][^>]*[^/]>$/) && !trimmed.startsWith('<!')) {
        const tagMatch = trimmed.match(/<(\w+)/);
        if (tagMatch) {
          const tag = tagMatch[1].toLowerCase();
          const voidTags = ['br', 'hr', 'img', 'input', 'meta', 'link', 'area', 'base', 'col', 'embed'];
          if (!voidTags.includes(tag)) {
            depth++;
          }
        }
      }
    }

    return result.join('\n');
  }

  // Helper to format script content
  function formatScriptContent(content: string, indent: string, semi: boolean, singleQuote: boolean, bracketSpacing: boolean): string {
    let formatted = content;

    // Normalize indentation
    const lines = formatted.split('\n');
    const result: string[] = [];
    let depth = 0;

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) {
        result.push('');
        continue;
      }

      // Track braces for indentation
      const openBraces = (trimmed.match(/[{([\[]/g) || []).length;
      const closeBraces = (trimmed.match(/[})\]]/g) || []).length;

      // Decrease indent for closing braces at start
      if (trimmed.match(/^[}\])]/) || trimmed.match(/^<\/|^\)/)) {
        depth = Math.max(0, depth - 1);
      }

      result.push(indent.repeat(depth) + trimmed);

      // Adjust depth based on braces
      depth += openBraces - closeBraces;
      depth = Math.max(0, depth);
    }

    formatted = result.join('\n');

    // Handle semicolons
    if (!semi) {
      formatted = formatted.replace(/;(\s*\n)/g, '$1');
    }

    // Handle quotes
    if (singleQuote) {
      formatted = formatted.replace(/"([^"\\]*(?:\\.[^"\\]*)*)"/g, "'$1'");
    }

    // Handle bracket spacing in objects
    if (bracketSpacing) {
      formatted = formatted.replace(/\{(\S)/g, '{ $1');
      formatted = formatted.replace(/(\S)\}/g, '$1 }');
    } else {
      formatted = formatted.replace(/\{\s+/g, '{');
      formatted = formatted.replace(/\s+\}/g, '}');
    }

    return formatted;
  }

  // Helper to format style content
  function formatStyleContent(content: string, indent: string): string {
    const lines = content.split('\n');
    const result: string[] = [];
    let depth = 0;

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) continue;

      // Decrease indent for closing braces
      if (trimmed.startsWith('}')) {
        depth = Math.max(0, depth - 1);
      }

      result.push(indent.repeat(depth) + trimmed);

      // Increase indent after opening braces
      if (trimmed.endsWith('{')) {
        depth++;
      }
    }

    return result.join('\n');
  }

  const mockFormatTemplate = (source: string, options: FormatOptions): FormatResult => {
    const tabWidth = options.tabWidth || 2;
    const useTabs = options.useTabs || false;
    const indent = useTabs ? '\t' : ' '.repeat(tabWidth);

    // Basic template formatting
    let formatted = source.replace(/\r\n/g, '\n');

    // Normalize indentation
    const lines = formatted.split('\n');
    const formattedLines: string[] = [];
    let depth = 0;

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) {
        formattedLines.push('');
        continue;
      }

      // Decrease indent for closing tags
      if (trimmed.startsWith('</')) {
        depth = Math.max(0, depth - 1);
      }

      formattedLines.push(indent.repeat(depth) + trimmed);

      // Increase indent after opening tags (not self-closing)
      if (trimmed.startsWith('<') && !trimmed.startsWith('</') && !trimmed.endsWith('/>') && !trimmed.startsWith('<!')) {
        const tagMatch = trimmed.match(/<(\w+)/);
        if (tagMatch) {
          const tag = tagMatch[1].toLowerCase();
          // Skip void elements
          const voidTags = ['br', 'hr', 'img', 'input', 'meta', 'link', 'area', 'base', 'col', 'embed', 'param', 'source', 'track', 'wbr'];
          if (!voidTags.includes(tag)) {
            depth++;
          }
        }
      }
    }

    formatted = formattedLines.join('\n');

    return {
      code: formatted,
      changed: formatted !== source,
    };
  };

  const mockFormatScript = (source: string, options: FormatOptions): FormatResult => {
    const tabWidth = options.tabWidth || 2;
    const useTabs = options.useTabs || false;
    const indent = useTabs ? '\t' : ' '.repeat(tabWidth);
    const semi = options.semi !== false;
    const singleQuote = options.singleQuote || false;

    let formatted = source.replace(/\r\n/g, '\n');

    // Basic formatting
    if (singleQuote) {
      // Convert double quotes to single quotes (simplified)
      formatted = formatted.replace(/"([^"\\]*(?:\\.[^"\\]*)*)"/g, "'$1'");
    }

    // Handle semicolons
    if (!semi) {
      formatted = formatted.replace(/;(\s*\n)/g, '$1');
    }

    // Normalize indentation
    formatted = formatted.replace(/\t/g, indent);

    return {
      code: formatted,
      changed: formatted !== source,
    };
  };

  const mockGetLocales = (): LocaleInfo[] => {
    return [
      { code: 'en', name: 'English' },
      { code: 'ja', name: '日本語' },
      { code: 'zh', name: '中文' },
    ];
  };

  const mockAnalyzeSfc = (source: string, _options: CroquisOptions): CroquisResult => {
    // Parse the SFC to extract information
    const hasScriptSetup = source.includes('<script setup');
    const hasDefineProps = source.includes('defineProps');
    const hasDefineEmits = source.includes('defineEmits');
    const hasScoped = source.includes('<style scoped');

    // Extract template content for checking if bindings are used
    const templateMatch = source.match(/<template>([\s\S]*?)<\/template>/);
    const templateContent = templateMatch ? templateMatch[1] : '';

    // Helper to check if a binding is used in template
    const isUsedInTemplate = (name: string): boolean => {
      // Check for {{ name }}, :prop="name", @event="name", v-bind:x="name", etc.
      const patterns = [
        new RegExp(`\\{\\{[^}]*\\b${name}\\b[^}]*\\}\\}`, 'g'),  // {{ name }}
        new RegExp(`:[a-z-]+="[^"]*\\b${name}\\b[^"]*"`, 'gi'),  // :prop="name"
        new RegExp(`@[a-z-]+="[^"]*\\b${name}\\b[^"]*"`, 'gi'),  // @event="name"
        new RegExp(`v-[a-z]+="[^"]*\\b${name}\\b[^"]*"`, 'gi'),  // v-xxx="name"
      ];
      return patterns.some(p => p.test(templateContent));
    };

    const bindings: BindingDisplay[] = [];
    const macros: MacroDisplay[] = [];
    const props: PropDisplay[] = [];
    const emits: EmitDisplay[] = [];
    const typeExports: TypeExportDisplay[] = [];
    const invalidExports: InvalidExportDisplay[] = [];

    // Extract type exports (export type / export interface) - valid in script setup
    const typeExportRegex = /export\s+type\s+(\w+)\s*=/g;
    let typeMatch;
    while ((typeMatch = typeExportRegex.exec(source)) !== null) {
      typeExports.push({
        name: typeMatch[1],
        kind: 'type',
        start: typeMatch.index,
        end: typeMatch.index + typeMatch[0].length,
        hoisted: true,
      });
    }

    const interfaceExportRegex = /export\s+interface\s+(\w+)\s*\{/g;
    while ((typeMatch = interfaceExportRegex.exec(source)) !== null) {
      typeExports.push({
        name: typeMatch[1],
        kind: 'interface',
        start: typeMatch.index,
        end: typeMatch.index + typeMatch[0].length,
        hoisted: true,
      });
    }

    // Extract invalid exports (const/let/var/function/class/default) - invalid in script setup
    if (hasScriptSetup) {
      // Get the script setup content only
      const scriptSetupMatch = source.match(/<script[^>]*setup[^>]*>([\s\S]*?)<\/script>/);
      if (scriptSetupMatch) {
        const setupContent = scriptSetupMatch[1];
        const setupStart = source.indexOf(scriptSetupMatch[0]) + scriptSetupMatch[0].indexOf('>') + 1;

        // export const
        const exportConstRegex = /export\s+const\s+(\w+)/g;
        let exportMatch;
        while ((exportMatch = exportConstRegex.exec(setupContent)) !== null) {
          invalidExports.push({
            name: exportMatch[1],
            kind: 'const',
            start: setupStart + exportMatch.index,
            end: setupStart + exportMatch.index + exportMatch[0].length,
          });
        }

        // export let
        const exportLetRegex = /export\s+let\s+(\w+)/g;
        while ((exportMatch = exportLetRegex.exec(setupContent)) !== null) {
          invalidExports.push({
            name: exportMatch[1],
            kind: 'let',
            start: setupStart + exportMatch.index,
            end: setupStart + exportMatch.index + exportMatch[0].length,
          });
        }

        // export var
        const exportVarRegex = /export\s+var\s+(\w+)/g;
        while ((exportMatch = exportVarRegex.exec(setupContent)) !== null) {
          invalidExports.push({
            name: exportMatch[1],
            kind: 'var',
            start: setupStart + exportMatch.index,
            end: setupStart + exportMatch.index + exportMatch[0].length,
          });
        }

        // export function (but not export type)
        const exportFunctionRegex = /export\s+(?:async\s+)?function\s+(\w+)/g;
        while ((exportMatch = exportFunctionRegex.exec(setupContent)) !== null) {
          invalidExports.push({
            name: exportMatch[1],
            kind: 'function',
            start: setupStart + exportMatch.index,
            end: setupStart + exportMatch.index + exportMatch[0].length,
          });
        }

        // export class
        const exportClassRegex = /export\s+class\s+(\w+)/g;
        while ((exportMatch = exportClassRegex.exec(setupContent)) !== null) {
          invalidExports.push({
            name: exportMatch[1],
            kind: 'class',
            start: setupStart + exportMatch.index,
            end: setupStart + exportMatch.index + exportMatch[0].length,
          });
        }

        // export default
        const exportDefaultRegex = /export\s+default\s+/g;
        while ((exportMatch = exportDefaultRegex.exec(setupContent)) !== null) {
          invalidExports.push({
            name: 'default',
            kind: 'default',
            start: setupStart + exportMatch.index,
            end: setupStart + exportMatch.index + exportMatch[0].length,
          });
        }
      }
    }

    // Extract ref bindings
    const refMatches2 = source.matchAll(/const\s+(\w+)\s*=\s*ref\(/g);
    for (const match of refMatches2) {
      const name = match[1];
      const usedInTpl = isUsedInTemplate(name);
      bindings.push({
        name,
        kind: 'SetupRef',
        source: 'ref' as BindingSource,
        metadata: {
          isExported: false,
          isImported: false,
          isComponent: false,
          isDirective: false,
          needsValue: true,
          usedInTemplate: usedInTpl,
          usedInScript: true,
          scopeDepth: 0,
        },
        start: match.index || 0,
        end: (match.index || 0) + match[0].length,
        isUsed: true,
        isMutated: true,
        referenceCount: 1,
        bindable: true,
        usedInTemplate: usedInTpl,
        fromScriptSetup: true,
      });
    }

    // Extract computed bindings
    const computedMatches = source.matchAll(/const\s+(\w+)\s*=\s*computed\(/g);
    for (const match of computedMatches) {
      const name = match[1];
      const usedInTpl = isUsedInTemplate(name);
      bindings.push({
        name,
        kind: 'SetupComputed',
        source: 'computed' as BindingSource,
        metadata: {
          isExported: false,
          isImported: false,
          isComponent: false,
          isDirective: false,
          needsValue: true,
          usedInTemplate: usedInTpl,
          usedInScript: true,
          scopeDepth: 0,
        },
        start: match.index || 0,
        end: (match.index || 0) + match[0].length,
        isUsed: true,
        isMutated: false,
        referenceCount: 1,
        bindable: true,
        usedInTemplate: usedInTpl,
        fromScriptSetup: true,
      });
    }

    // Extract function bindings
    const functionMatches = source.matchAll(/function\s+(\w+)\s*\(/g);
    for (const match of functionMatches) {
      const name = match[1];
      const usedInTpl = isUsedInTemplate(name);
      bindings.push({
        name,
        kind: 'SetupConst',
        source: 'function' as BindingSource,
        metadata: {
          isExported: false,
          isImported: false,
          isComponent: false,
          isDirective: false,
          needsValue: false,
          usedInTemplate: usedInTpl,
          usedInScript: true,
          scopeDepth: 0,
        },
        start: match.index || 0,
        end: (match.index || 0) + match[0].length,
        isUsed: true,
        isMutated: false,
        referenceCount: 1,
        bindable: true,
        usedInTemplate: usedInTpl,
        fromScriptSetup: true,
      });
    }

    // Extract defineProps
    if (hasDefineProps) {
      // Match both inline types: defineProps<{...}>() and type references: defineProps<TypeName>()
      const propsMatch = source.match(/defineProps<([^>]+)>\s*\(\s*\)/);
      if (propsMatch) {
        const typeArg = propsMatch[1].trim();
        macros.push({
          name: 'defineProps',
          start: propsMatch.index || 0,
          end: (propsMatch.index || 0) + propsMatch[0].length,
          type_args: typeArg,
        });
        // Extract prop names from inline type (if it's an object type)
        if (typeArg.startsWith('{')) {
          const propNameMatches = typeArg.matchAll(/(\w+)(\?)?:/g);
          for (const propMatch of propNameMatches) {
            props.push({
              name: propMatch[1],
              required: !propMatch[2],
              has_default: false,
            });
          }
        }
      }
    }

    // Extract defineEmits
    if (hasDefineEmits) {
      // Match both inline types: defineEmits<{...}>() and type references: defineEmits<TypeName>()
      const emitsMatch = source.match(/defineEmits<([^>]+)>\s*\(\s*\)/);
      if (emitsMatch) {
        const typeArg = emitsMatch[1].trim();
        macros.push({
          name: 'defineEmits',
          start: emitsMatch.index || 0,
          end: (emitsMatch.index || 0) + emitsMatch[0].length,
          type_args: typeArg,
        });
        // Extract emit names from inline type (if it's an object type)
        if (typeArg.startsWith('{')) {
          const emitNameMatches = typeArg.matchAll(/(\w+):/g);
          for (const emitMatch of emitNameMatches) {
            emits.push({
              name: emitMatch[1],
            });
          }
        }
      }
    }

    // Generate VIR text (formal TOML-like format)
    let vir = '';
    vir += '# VIR v0.1\n';
    vir += '\n';

    // Stats section
    vir += '[stats]\n';
    vir += `bindings = ${bindings.length}\n`;
    vir += `macros = ${macros.length}\n`;

    // Macros section
    if (macros.length > 0) {
      vir += '\n[macros]\n';
      for (const macro of macros) {
        vir += `@${macro.name}`;
        if (macro.type_args) {
          vir += `<{\n${macro.type_args.trim()}\n}>`;
        }
        if (macro.identifier) {
          vir += ` -> ${macro.identifier}`;
        }
        vir += ` # ${macro.start}:${macro.end}\n`;
      }
    }

    // Bindings section
    if (bindings.length > 0) {
      vir += '\n[bindings]\n';
      for (const binding of bindings) {
        const flags: string[] = [];
        if (binding.kind === 'SetupRef') flags.push('ref');
        if (binding.metadata?.isUsed !== false) flags.push('used');
        if (binding.metadata?.isMutated) flags.push('mut');
        vir += `${binding.name}: ${binding.source} @0:0`;
        if (flags.length > 0) {
          vir += ` [${flags.join(', ')}]`;
        }
        vir += '\n';
      }
    }

    // CSS section
    const selectorCount = (source.match(/[.#\w][\w-]*\s*\{/g) || []).length;
    const vBindCount = (source.match(/v-bind\(/g) || []).length;
    if (hasScoped) {
      vir += '\n[css]\n';
      vir += `scoped = true\n`;
      vir += `selectors = ${selectorCount}\n`;
      vir += `v_bind = ${vBindCount}\n`;
    }

    // Props section
    if (props.length > 0) {
      vir += '\n[props]\n';
      for (const prop of props) {
        vir += `${prop.name}: ${prop.type || 'any'}\n`;
      }
    }

    // Emits section
    if (emits.length > 0) {
      vir += '\n[emits]\n';
      for (const emit of emits) {
        vir += `${emit.name}\n`;
      }
    }

    // Generate scopes from source analysis
    const scopes: ScopeDisplay[] = [];
    let scopeId = 0;

    // Helper function to strip comments from code (for accurate parsing)
    const stripComments = (code: string): string => {
      // Remove single-line comments
      let result = code.replace(/\/\/.*$/gm, '');
      // Remove multi-line comments
      result = result.replace(/\/\*[\s\S]*?\*\//g, '');
      return result;
    };

    // Helper function to extract declarations (functions, variables) from script content
    const extractDeclarations = (content: string): string[] => {
      const stripped = stripComments(content);
      const names: string[] = [];
      // Functions
      const funcRegex = /(?:export\s+)?(?:async\s+)?function\s+(\w+)/g;
      let match;
      while ((match = funcRegex.exec(stripped)) !== null) {
        names.push(match[1]);
      }
      // const/let/var declarations
      const varRegex = /(?:export\s+)?(?:const|let|var)\s+(\w+)/g;
      while ((match = varRegex.exec(stripped)) !== null) {
        names.push(match[1]);
      }
      return names;
    };

    // Helper function to extract imports from script content
    type ImportInfo = { name: string, path: string, start: number, end: number };
    const extractImports = (content: string, startOffset: number): { names: string[], externalImports: ImportInfo[] } => {
      const stripped = stripComments(content);
      const names: string[] = [];
      const externalImports: ImportInfo[] = [];
      const importRegex = /import\s+(?:type\s+)?(?:(\w+)|{\s*([^}]+)\s*}|\*\s+as\s+(\w+))?\s*(?:,\s*{\s*([^}]+)\s*})?\s*from\s+['"]([^'"]+)['"]/g;
      let match;
      while ((match = importRegex.exec(stripped)) !== null) {
        const defaultImport = match[1];
        const namedImports = match[2];
        const namespaceImport = match[3];
        const additionalNamed = match[4];
        const modulePath = match[5];

        const importedNames: string[] = [];
        if (defaultImport) importedNames.push(defaultImport);
        if (namespaceImport) importedNames.push(namespaceImport);
        if (namedImports) {
          const parsed = namedImports.split(',').map(n => n.trim().split(/\s+as\s+/).pop()?.trim()).filter(Boolean) as string[];
          importedNames.push(...parsed);
        }
        if (additionalNamed) {
          const parsed = additionalNamed.split(',').map(n => n.trim().split(/\s+as\s+/).pop()?.trim()).filter(Boolean) as string[];
          importedNames.push(...parsed);
        }

        names.push(...importedNames);

        // Check if it's an external module (not relative path or alias)
        const isExternal = !modulePath.startsWith('.') && !modulePath.startsWith('@/');
        if (isExternal) {
          externalImports.push({
            name: importedNames.join(', ') || modulePath,
            path: modulePath,
            start: startOffset + match.index,
            end: startOffset + match.index + match[0].length,
          });
        }
      }
      return { names, externalImports };
    };

    // JS universal globals (available everywhere in both server and client)
    const jsuGlobals = [
      'console', 'Math', 'JSON', 'Date', 'Array', 'Object', 'String', 'Number',
      'Boolean', 'Symbol', 'BigInt', 'Map', 'Set', 'WeakMap', 'WeakSet',
      'Promise', 'Proxy', 'Reflect', 'Error', 'TypeError', 'RangeError',
      'parseInt', 'parseFloat', 'isNaN', 'isFinite', 'encodeURI', 'decodeURI',
      'encodeURIComponent', 'decodeURIComponent', 'undefined', 'NaN', 'Infinity',
    ];

    // JS server-only globals (Node.js)
    const jssGlobals = [
      'process', 'Buffer', '__dirname', '__filename', 'module', 'exports', 'require',
      'global', 'setImmediate', 'clearImmediate',
    ];

    // JS client-only globals (Browser)
    const clientGlobals = [
      'window', 'document', 'navigator', 'location', 'history', 'localStorage',
      'sessionStorage', 'fetch', 'XMLHttpRequest', 'WebSocket', 'Worker',
      'requestAnimationFrame', 'cancelAnimationFrame', 'setTimeout', 'clearTimeout',
      'setInterval', 'clearInterval', 'alert', 'confirm', 'prompt',
    ];

    // Vue globals (template-only)
    const vueGlobals = [
      '$refs', '$emit', '$attrs', '$slots', '$props', '$el', '$options',
      '$data', '$watch', '$nextTick', '$forceUpdate',
    ];

    // Track hoisted items for module scope
    const hoistedBindings: string[] = [];

    // Module scope (root) - bindings will be populated later with hoisted items
    const moduleScope: ScopeDisplay = {
      id: scopeId++,
      kind: 'mod',
      kindStr: 'Mod',
      start: 0,
      end: source.length,
      bindings: [], // Will be populated with hoisted items
      children: [],
      depth: 0,
    };
    scopes.push(moduleScope);

    // Detect non-script-setup block (regular <script> without setup attribute)
    const nonSetupScriptMatch = source.match(/<script(?![^>]*setup)[^>]*>([\s\S]*?)<\/script>/);
    if (nonSetupScriptMatch) {
      const nonSetupStart = source.indexOf(nonSetupScriptMatch[0]);
      const nonSetupEnd = nonSetupStart + nonSetupScriptMatch[0].length;
      const nonSetupContent = nonSetupScriptMatch[1];
      // contentStart is where the actual script content begins (after the opening tag)
      const nonSetupContentStart = nonSetupStart + nonSetupScriptMatch[0].indexOf('>') + 1;

      const { names: importNames, externalImports } = extractImports(nonSetupContent, nonSetupContentStart);
      const declNames = extractDeclarations(nonSetupContent);
      const allPlainBindings = [...new Set([...importNames, ...declNames])];

      // Add plain bindings to hoisted (module scope)
      hoistedBindings.push(...allPlainBindings);

      const nonSetupScope: ScopeDisplay = {
        id: scopeId++,
        parentIds: [0],
        kind: 'plain' as ScopeKind,
        kindStr: 'Plain',
        start: nonSetupStart,
        end: nonSetupEnd,
        bindings: allPlainBindings,
        children: [],
        depth: 1,
      };
      moduleScope.children.push(nonSetupScope.id);
      scopes.push(nonSetupScope);

      // Add external module scopes for imports
      for (const ext of externalImports) {
        const externalScope: ScopeDisplay = {
          id: scopeId++,
          parentIds: [nonSetupScope.id],
          kind: 'extern' as ScopeKind,
          kindStr: `Extern (${ext.path})`,
          start: ext.start,
          end: ext.end,
          bindings: ext.name.split(', ').filter(Boolean),
          children: [],
          depth: 2,
        };
        nonSetupScope.children.push(externalScope.id);
        scopes.push(externalScope);
      }
    }

    // Detect setup scope if script setup exists
    if (hasScriptSetup) {
      const scriptSetupMatch = source.match(/<script[^>]*setup[^>]*>([\s\S]*?)<\/script>/);
      if (scriptSetupMatch) {
        const setupStart = source.indexOf(scriptSetupMatch[0]);
        const setupEnd = setupStart + scriptSetupMatch[0].length;
        const setupContent = scriptSetupMatch[1];
        // Use stripped content for detection to avoid matching commented code
        const strippedSetupContent = stripComments(setupContent);
        // Calculate the actual content start (after the opening tag)
        const contentStart = setupStart + scriptSetupMatch[0].indexOf('>') + 1;

        // Extract imports from script setup
        const { names: setupImportNames, externalImports: setupExternalImports } = extractImports(setupContent, contentStart);

        // Add setup imports to hoisted (module scope)
        hoistedBindings.push(...setupImportNames);

        // Add export types to hoisted (they are already in typeExports)
        hoistedBindings.push(...typeExports.filter(t => t.hoisted).map(t => t.name));

        // Setup scope contains only directly defined bindings (not imports)
        const setupBindings = bindings.map(b => b.name);

        const setupScope: ScopeDisplay = {
          id: scopeId++,
          parentIds: [0],
          kind: 'setup' as ScopeKind,
          kindStr: 'Setup',
          start: setupStart,
          end: setupEnd,
          bindings: setupBindings, // Only directly defined bindings
          children: [],
          depth: 1,
        };
        moduleScope.children.push(setupScope.id);
        scopes.push(setupScope);

        // Add external module scopes for script setup imports
        for (const ext of setupExternalImports) {
          const externalScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'extern' as ScopeKind,
            kindStr: `Extern (${ext.path})`,
            start: ext.start,
            end: ext.end,
            bindings: ext.name.split(', ').filter(Boolean),
            children: [],
            depth: 2,
          };
          setupScope.children.push(externalScope.id);
          scopes.push(externalScope);
        }

        // Detect function scopes inside setup
        const functionRegex = /function\s+(\w+)\s*\([^)]*\)\s*\{/g;
        let funcMatch;
        while ((funcMatch = functionRegex.exec(setupContent)) !== null) {
          const funcScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'function',
            kindStr: `Function (${funcMatch[1]})`,
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 50,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(funcScope.id);
          scopes.push(funcScope);
        }

        // Detect arrow function scopes
        const arrowRegex = /const\s+(\w+)\s*=\s*\([^)]*\)\s*=>/g;
        while ((funcMatch = arrowRegex.exec(setupContent)) !== null) {
          const arrowScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: `Arrow (${funcMatch[1]})`,
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 50,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(arrowScope.id);
          scopes.push(arrowScope);
        }

        // Detect watch callbacks
        const watchRegex = /watch\s*\([^,]+,\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = watchRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const watchScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'watch',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(watchScope.id);
          scopes.push(watchScope);
        }

        // Detect watchEffect callbacks
        const watchEffectRegex = /watchEffect\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = watchEffectRegex.exec(setupContent)) !== null) {
          const watchEffectScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'watchEffect',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(watchEffectScope.id);
          scopes.push(watchEffectScope);
        }

        // Detect computed callbacks
        const computedRegex = /(?:const|let)\s+(\w+)\s*=\s*computed\s*\(\s*(?:\([^)]*\)\s*)?=>/g;
        while ((funcMatch = computedRegex.exec(setupContent)) !== null) {
          const computedScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: `computed (${funcMatch[1]})`,
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(computedScope.id);
          scopes.push(computedScope);
        }

        // Detect computed with getter/setter
        const computedGetSetRegex = /(?:const|let)\s+(\w+)\s*=\s*computed\s*\(\s*\{/g;
        while ((funcMatch = computedGetSetRegex.exec(setupContent)) !== null) {
          const computedScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'function',
            kindStr: `computed (${funcMatch[1]}) [get/set]`,
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 50,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(computedScope.id);
          scopes.push(computedScope);
        }

        // Detect lifecycle hooks - Client-only lifecycle hooks for SSR
        const clientOnlyHooks = ['onMounted', 'onUnmounted', 'onBeforeMount', 'onBeforeUnmount', 'onUpdated', 'onBeforeUpdate', 'onActivated', 'onDeactivated'];
        const universalHooks = ['onErrorCaptured', 'onRenderTracked', 'onRenderTriggered'];
        const serverOnlyHooks = ['onServerPrefetch'];

        // Client-only hooks - code inside runs only on client
        for (const hook of clientOnlyHooks) {
          const hookRegex = new RegExp(`${hook}\\s*\\(\\s*(?:async\\s*)?\\(?([^)]*)\\)?\\s*=>`, 'g');
          while ((funcMatch = hookRegex.exec(setupContent)) !== null) {
            const hookScope: ScopeDisplay = {
              id: scopeId++,
              parentIds: [setupScope.id],
              kind: 'client' as ScopeKind,
              kindStr: `ClientOnly (${hook})`,
              start: contentStart + funcMatch.index,
              end: contentStart + funcMatch.index + funcMatch[0].length + 30,
              bindings: [],
              children: [],
              depth: 2,
            };
            setupScope.children.push(hookScope.id);
            scopes.push(hookScope);
          }
        }

        // Universal hooks - code runs on both server and client
        for (const hook of universalHooks) {
          const hookRegex = new RegExp(`${hook}\\s*\\(\\s*(?:async\\s*)?\\(?([^)]*)\\)?\\s*=>`, 'g');
          while ((funcMatch = hookRegex.exec(setupContent)) !== null) {
            const hookScope: ScopeDisplay = {
              id: scopeId++,
              parentIds: [setupScope.id],
              kind: 'universal' as ScopeKind,
              kindStr: `Universal (${hook})`,
              start: contentStart + funcMatch.index,
              end: contentStart + funcMatch.index + funcMatch[0].length + 30,
              bindings: [],
              children: [],
              depth: 2,
            };
            setupScope.children.push(hookScope.id);
            scopes.push(hookScope);
          }
        }

        // Server-only hooks
        for (const hook of serverOnlyHooks) {
          const hookRegex = new RegExp(`${hook}\\s*\\(\\s*(?:async\\s*)?\\(?([^)]*)\\)?\\s*=>`, 'g');
          while ((funcMatch = hookRegex.exec(setupContent)) !== null) {
            const hookScope: ScopeDisplay = {
              id: scopeId++,
              parentIds: [setupScope.id],
              kind: 'function',
              kindStr: `ServerOnly (${hook})`,
              start: contentStart + funcMatch.index,
              end: contentStart + funcMatch.index + funcMatch[0].length + 30,
              bindings: [],
              children: [],
              depth: 2,
            };
            setupScope.children.push(hookScope.id);
            scopes.push(hookScope);
          }
        }

        // Detect provide with factory function
        const provideRegex = /provide\s*\(\s*['"][^'"]+['"]\s*,\s*\(\)\s*=>/g;
        while ((funcMatch = provideRegex.exec(setupContent)) !== null) {
          const provideScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'provide factory',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(provideScope.id);
          scopes.push(provideScope);
        }

        // Detect inject with default factory
        const injectRegex = /inject\s*\(\s*['"][^'"]+['"]\s*,\s*\(\)\s*=>/g;
        while ((funcMatch = injectRegex.exec(setupContent)) !== null) {
          const injectScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'inject default',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(injectScope.id);
          scopes.push(injectScope);
        }

        // Detect try-catch blocks
        const tryCatchRegex = /try\s*\{/g;
        while ((funcMatch = tryCatchRegex.exec(setupContent)) !== null) {
          const tryScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'block',
            kindStr: 'try',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(tryScope.id);
          scopes.push(tryScope);
        }

        const catchRegex = /catch\s*\(\s*(\w+)\s*\)\s*\{/g;
        while ((funcMatch = catchRegex.exec(setupContent)) !== null) {
          const catchScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'block',
            kindStr: 'catch',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [funcMatch[1]],
            children: [],
            depth: 2,
          };
          setupScope.children.push(catchScope.id);
          scopes.push(catchScope);
        }

        // Detect for loops
        const forLoopRegex = /for\s*\(\s*(?:const|let|var)\s+(\w+)/g;
        while ((funcMatch = forLoopRegex.exec(setupContent)) !== null) {
          const forScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'block',
            kindStr: 'for',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [funcMatch[1]],
            children: [],
            depth: 2,
          };
          setupScope.children.push(forScope.id);
          scopes.push(forScope);
        }

        // Detect for...of / for...in loops
        const forOfInRegex = /for\s*\(\s*(?:const|let|var)\s+(\w+)\s+(?:of|in)\s+/g;
        while ((funcMatch = forOfInRegex.exec(setupContent)) !== null) {
          const forOfScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'block',
            kindStr: 'for..of/in',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [funcMatch[1]],
            children: [],
            depth: 2,
          };
          setupScope.children.push(forOfScope.id);
          scopes.push(forOfScope);
        }

        // Detect if blocks with block-scoped variables
        const ifLetRegex = /if\s*\([^)]+\)\s*\{[^}]*(?:const|let)\s+(\w+)/g;
        while ((funcMatch = ifLetRegex.exec(setupContent)) !== null) {
          const ifScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'block',
            kindStr: 'if',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: [funcMatch[1]],
            children: [],
            depth: 2,
          };
          setupScope.children.push(ifScope.id);
          scopes.push(ifScope);
        }

        // Detect Array.forEach callbacks
        const forEachRegex = /\.forEach\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = forEachRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const forEachScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'forEach',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(forEachScope.id);
          scopes.push(forEachScope);
        }

        // Detect Array.map callbacks
        const mapRegex = /\.map\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = mapRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const mapScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'map',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(mapScope.id);
          scopes.push(mapScope);
        }

        // Detect Array.filter callbacks
        const filterRegex = /\.filter\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = filterRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const filterScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'filter',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(filterScope.id);
          scopes.push(filterScope);
        }

        // Detect Array.reduce callbacks
        const reduceRegex = /\.reduce\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = reduceRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const reduceScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'reduce',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(reduceScope.id);
          scopes.push(reduceScope);
        }

        // Detect Array.find/findIndex callbacks
        const findRegex = /\.find(?:Index)?\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = findRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const findScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'find',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(findScope.id);
          scopes.push(findScope);
        }

        // Detect Array.some/every callbacks
        const someEveryRegex = /\.(?:some|every)\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = someEveryRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const someEveryScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'some/every',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(someEveryScope.id);
          scopes.push(someEveryScope);
        }

        // Detect Promise.then callbacks
        const thenRegex = /\.then\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = thenRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const thenScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: '.then',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(thenScope.id);
          scopes.push(thenScope);
        }

        // Detect Promise.catch callbacks
        const promiseCatchRegex = /\.catch\s*\(\s*\(?([^)]*)\)?\s*=>/g;
        while ((funcMatch = promiseCatchRegex.exec(setupContent)) !== null) {
          const params = funcMatch[1]?.split(',').map(p => p.trim()).filter(Boolean) || [];
          const promiseCatchScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: '.catch',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: params,
            children: [],
            depth: 2,
          };
          setupScope.children.push(promiseCatchScope.id);
          scopes.push(promiseCatchScope);
        }

        // Detect Promise.finally callbacks
        const finallyRegex = /\.finally\s*\(\s*\(\)\s*=>/g;
        while ((funcMatch = finallyRegex.exec(setupContent)) !== null) {
          const finallyScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: '.finally',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(finallyScope.id);
          scopes.push(finallyScope);
        }

        // Detect setTimeout/setInterval callbacks
        const timerRegex = /set(?:Timeout|Interval)\s*\(\s*\(\)\s*=>/g;
        while ((funcMatch = timerRegex.exec(setupContent)) !== null) {
          const timerScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'timer',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(timerScope.id);
          scopes.push(timerScope);
        }

        // Detect nextTick callbacks
        const nextTickRegex = /nextTick\s*\(\s*\(\)\s*=>/g;
        while ((funcMatch = nextTickRegex.exec(setupContent)) !== null) {
          const nextTickScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'nextTick',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 20,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(nextTickScope.id);
          scopes.push(nextTickScope);
        }

        // Detect async IIFE
        const asyncIifeRegex = /\(\s*async\s*\(\)\s*=>\s*\{/g;
        while ((funcMatch = asyncIifeRegex.exec(setupContent)) !== null) {
          const asyncIifeScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [setupScope.id],
            kind: 'arrowFunction',
            kindStr: 'async IIFE',
            start: contentStart + funcMatch.index,
            end: contentStart + funcMatch.index + funcMatch[0].length + 30,
            bindings: [],
            children: [],
            depth: 2,
          };
          setupScope.children.push(asyncIifeScope.id);
          scopes.push(asyncIifeScope);
        }
      }
    }

    // Detect v-for scopes in template (reuse templateMatch from above)
    if (templateMatch) {
      const templateTagStart = source.indexOf(templateMatch[0]);
      // templateContentStart is where the actual template content begins (after the opening tag)
      const templateContentStart = templateTagStart + templateMatch[0].indexOf('>') + 1;
      const vForRegex = /v-for="([^"]+)"/g;
      let vForMatch;
      while ((vForMatch = vForRegex.exec(templateMatch[1])) !== null) {
        const expr = vForMatch[1];
        const inMatch = expr.match(/\(?([^)]+)\)?\s+(?:in|of)\s+/);
        const vForBindings: string[] = [];
        if (inMatch) {
          const aliases = inMatch[1].split(',').map(s => s.trim());
          vForBindings.push(...aliases);
        }

        const vForScope: ScopeDisplay = {
          id: scopeId++,
          parentIds: [0],  // Will add vue_global later
          kind: 'vFor',
          kindStr: `v-for`,
          start: templateContentStart + vForMatch.index,
          end: templateContentStart + vForMatch.index + vForMatch[0].length,
          bindings: vForBindings,
          children: [],
          depth: 1,
        };
        (vForScope as any)._isTemplateScope = true;  // Mark for vue_global parent addition
        moduleScope.children.push(vForScope.id);
        scopes.push(vForScope);
      }

      // Detect v-slot scopes
      const vSlotRegex = /v-slot(?::(\w+))?="([^"]+)"/g;
      let vSlotMatch;
      while ((vSlotMatch = vSlotRegex.exec(templateMatch[1])) !== null) {
        const slotName = vSlotMatch[1] || 'default';
        const slotParams = vSlotMatch[2]?.match(/\{?\s*([^}]+)\s*\}?/)?.[1]?.split(',').map(s => s.trim()) || [];
        const vSlotScope: ScopeDisplay = {
          id: scopeId++,
          parentIds: [0],  // Will add vue_global later
          kind: 'vSlot',
          kindStr: `v-slot:${slotName}`,
          start: templateContentStart + vSlotMatch.index,
          end: templateContentStart + vSlotMatch.index + vSlotMatch[0].length,
          bindings: slotParams,
          children: [],
          depth: 1,
        };
        (vSlotScope as any)._isTemplateScope = true;  // Mark for vue_global parent addition
        moduleScope.children.push(vSlotScope.id);
        scopes.push(vSlotScope);
      }

      // Detect inline event handler scopes
      const eventRegex = /@(\w+)="([^"]+)"/g;
      let eventMatch;
      while ((eventMatch = eventRegex.exec(templateMatch[1])) !== null) {
        const handler = eventMatch[2];
        if (handler.includes('=>') || handler.includes('$event')) {
          const eventScope: ScopeDisplay = {
            id: scopeId++,
            parentIds: [0],  // Will add vue_global later
            kind: 'arrowFunction',
            kindStr: `@${eventMatch[1]} handler`,
            start: templateContentStart + eventMatch.index,
            end: templateContentStart + eventMatch.index + eventMatch[0].length,
            bindings: ['$event'],
            children: [],
            depth: 1,
          };
          (eventScope as any)._isTemplateScope = true;  // Mark for vue_global parent addition
          moduleScope.children.push(eventScope.id);
          scopes.push(eventScope);
        }
      }
    }

    // Add global scopes (these are implicit, no parent)
    // Order: ~0 = js_global (universal), ~1 = vue_global, ~2 = mod, ...
    const universalScope: ScopeDisplay = {
      id: scopeId++,
      parentIds: [],
      kind: 'universal' as ScopeKind,
      kindStr: 'JsGlobal',
      start: 0,
      end: 0,
      bindings: jsuGlobals,
      children: [],
      depth: 0,
    };
    scopes.unshift(universalScope);  // ~0

    const vueScope: ScopeDisplay = {
      id: scopeId++,
      parentIds: [],
      kind: 'vue' as ScopeKind,
      kindStr: 'Vue',
      start: 0,
      end: 0,
      bindings: vueGlobals,
      children: [],
      depth: 0,
    };
    scopes.splice(1, 0, vueScope);  // Insert at position 1 -> ~1

    const serverScope: ScopeDisplay = {
      id: scopeId++,
      parentIds: [],
      kind: 'server' as ScopeKind,
      kindStr: 'Server',
      start: 0,
      end: 0,
      bindings: jssGlobals,
      children: [],
      depth: 0,
    };
    scopes.push(serverScope);

    const clientScope: ScopeDisplay = {
      id: scopeId++,
      parentIds: [],
      kind: 'client' as ScopeKind,
      kindStr: 'Client',
      start: 0,
      end: 0,
      bindings: clientGlobals,
      children: [],
      depth: 0,
    };
    scopes.push(clientScope);

    // Add vue_global as parent for template scopes (vFor, vSlot, event handlers)
    for (const scope of scopes) {
      if ((scope as any)._isTemplateScope && scope.parentIds) {
        scope.parentIds.push(vueScope.id);
        delete (scope as any)._isTemplateScope;
      }
    }

    // Populate module scope bindings with hoisted items + jsu globals
    moduleScope.bindings = [...new Set([...hoistedBindings, ...jsuGlobals])];

    // Build scope map for O(1) parent lookup
    const scopeMap = new Map<number, ScopeDisplay>();
    for (const s of scopes) scopeMap.set(s.id, s);

    // Get prefix for scope kind (for index and parent references)
    // - `~` = universal (works on both client and server)
    // - `!` = client only (requires client API: window, document, etc.)
    // - `#` = server private (reserved for future Server Components)
    const getScopePrefix = (kind: string): string => {
      switch (kind) {
        case 'client': return '!';
        case 'server': return '#';
        default: return '~';
      }
    };

    // Assign display IDs per prefix type (separate counters for #, ~, !)
    const prefixCounters: Record<string, number> = { '#': 0, '~': 0, '!': 0 };
    const displayIdMap = new Map<number, string>();  // internal id -> "prefix + displayId"
    for (const scope of scopes) {
      const prefix = getScopePrefix(scope.kind);
      const displayId = prefixCounters[prefix]++;
      displayIdMap.set(scope.id, `${prefix}${displayId}`);
    }

    // Get parent references: $ #1, ~4 (comma-separated with display IDs)
    const getParentRefs = (parentIds: number[]): string => {
      if (!parentIds || parentIds.length === 0) return '';
      const refs = parentIds.map(pid => displayIdMap.get(pid) || `#${pid}`);
      return ` $ ${refs.join(', ')}`;
    };

    // Add scopes to VIR
    if (scopes.length > 0) {
      vir += '\n[scopes]\n';
      for (const scope of scopes) {
        const displayId = displayIdMap.get(scope.id) || `#${scope.id}`;
        vir += `${displayId} ${scope.kindStr.toLowerCase()} @${scope.start}:${scope.end}`;
        if (scope.bindings.length > 0) {
          vir += ` {${scope.bindings.join(', ')}}`;
        }
        if (scope.parentIds && scope.parentIds.length > 0) {
          vir += getParentRefs(scope.parentIds);
        }
        vir += '\n';
      }
    }

    // Update stats with scope count
    vir = vir.replace('[stats]\n', `[stats]\nscopes = ${scopes.length}\n`);

    // Add type exports to VIR
    if (typeExports.length > 0) {
      vir += '\n[type_exports]\n';
      for (const te of typeExports) {
        vir += `${te.kind} ${te.name} @${te.start}:${te.end} [hoisted]\n`;
      }
    }

    // Add invalid exports to VIR
    if (invalidExports.length > 0) {
      vir += '\n[invalid_exports]\n';
      for (const ie of invalidExports) {
        vir += `${ie.kind} ${ie.name} @${ie.start}:${ie.end} [INVALID]\n`;
      }
    }

    const croquis: Croquis = {
      is_setup: hasScriptSetup,
      bindings,
      scopes,
      macros,
      props,
      emits,
      typeExports,
      invalidExports,
      css: hasScoped ? {
        selector_count: (source.match(/[.#\w][\w-]*\s*\{/g) || []).length,
        unused_selectors: [],
        v_bind_count: (source.match(/v-bind\(/g) || []).length,
        is_scoped: true,
      } : undefined,
      diagnostics: [],
      stats: {
        binding_count: bindings.length,
        unused_binding_count: 0,
        scope_count: scopes.length,
        macro_count: macros.length,
        error_count: 0,
        warning_count: 0,
        type_export_count: typeExports.length,
        invalid_export_count: invalidExports.length,
      },
    };

    return {
      croquis,
      diagnostics: [],
      vir,
    };
  };

  // Mock typeCheck function
  const mockTypeCheck = (source: string, options: TypeCheckOptions): TypeCheckResult => {
    const diagnostics: TypeCheckDiagnostic[] = [];
    const filename = options.filename || 'anonymous.vue';
    const strict = options.strict || false;

    // Check for untyped props (if enabled)
    if (options.checkProps !== false) {
      const hasDefineProps = source.includes('defineProps');
      const hasTypedProps = source.includes('defineProps<') || source.includes('defineProps({');
      if (hasDefineProps && !hasTypedProps) {
        const propsMatch = source.match(/defineProps\(/);
        if (propsMatch) {
          diagnostics.push({
            severity: strict ? 'error' : 'warning',
            message: 'Props should have a type definition',
            start: propsMatch.index || 0,
            end: (propsMatch.index || 0) + 12,
            code: 'untyped-prop',
            help: 'Use defineProps<{ propName: Type }>() or define runtime type',
            related: [],
          });
        }
      }
    }

    // Check for untyped emits (if enabled)
    if (options.checkEmits !== false) {
      const hasDefineEmits = source.includes('defineEmits');
      const hasTypedEmits = source.includes('defineEmits<') || source.includes('defineEmits([');
      if (hasDefineEmits && !hasTypedEmits) {
        const emitsMatch = source.match(/defineEmits\(/);
        if (emitsMatch) {
          diagnostics.push({
            severity: strict ? 'error' : 'warning',
            message: 'Emits should have a type definition',
            start: emitsMatch.index || 0,
            end: (emitsMatch.index || 0) + 12,
            code: 'untyped-emit',
            help: 'Use defineEmits<{ event: [payload: Type] }>()',
            related: [],
          });
        }
      }
    }

    const errorCount = diagnostics.filter(d => d.severity === 'error').length;
    const warningCount = diagnostics.filter(d => d.severity === 'warning').length;

    return {
      diagnostics,
      virtualTs: options.includeVirtualTs ? `// Virtual TypeScript for ${filename}\n// (mock)` : undefined,
      errorCount,
      warningCount,
      analysisTimeMs: 0.5,
    };
  };

  const mockGetTypeCheckCapabilities = (): TypeCheckCapabilities => {
    return {
      mode: 'ast-based',
      description: 'AST-based type analysis (no TypeScript compiler required)',
      checks: [
        {
          name: 'untyped-props',
          description: 'Detects props without type definitions',
          severity: 'warning',
        },
        {
          name: 'untyped-emits',
          description: 'Detects emits without type definitions',
          severity: 'warning',
        },
        {
          name: 'undefined-binding',
          description: 'Detects undefined template bindings',
          severity: 'error',
        },
      ],
      notes: [
        'For full TypeScript type checking, use the CLI with tsgo integration',
        'AST-based analysis catches common issues without external dependencies',
      ],
    };
  };

  // Mock cross-file analyzer (returns empty results)
  const mockAnalyzeCrossFile = (_files: CrossFileInput[], _options: CrossFileOptions): CrossFileResult => ({
    diagnostics: [],
    circularDependencies: [],
    stats: {
      filesAnalyzed: 0,
      vueComponents: 0,
      dependencyEdges: 0,
      errorCount: 0,
      warningCount: 0,
      infoCount: 0,
      analysisTimeMs: 0,
    },
    filePaths: [],
  });

  return {
    compile: mockCompile,
    compileVapor: (template: string, options: CompilerOptions) =>
      mockCompile(template, { ...options, outputMode: 'vapor' }),
    compileCss: mockCompileCss,
    parseTemplate: (template: string) => buildMockAst(template),
    parseSfc: mockParseSfc,
    compileSfc: mockCompileSfc,
    analyzeSfc: mockAnalyzeSfc,
    analyzeCrossFile: mockAnalyzeCrossFile,
    parseArt: mockParseArt,
    artToCsf: mockArtToCsf,
    lintTemplate: mockLintTemplate,
    lintSfc: mockLintSfc,
    getLintRules: mockGetLintRules,
    getLocales: mockGetLocales,
    formatSfc: mockFormatSfc,
    formatTemplate: mockFormatTemplate,
    formatScript: mockFormatScript,
    typeCheck: mockTypeCheck,
    getTypeCheckCapabilities: mockGetTypeCheckCapabilities,
    Compiler: MockCompiler,
  };
}

function buildMockAst(template: string): object {
  const children: object[] = [];

  // Simple regex-based parsing for mock AST
  const elementMatch = template.match(/<(\w+)([^>]*)>([\s\S]*?)<\/\1>/);
  if (elementMatch) {
    const [, tag, attrs, content] = elementMatch;
    const props: object[] = [];

    // Parse attributes
    const vIfMatch = attrs.match(/v-if="([^"]+)"/);
    if (vIfMatch) {
      props.push({ type: 'DIRECTIVE', name: 'if', exp: vIfMatch[1] });
    }

    const vForMatch = attrs.match(/v-for="([^"]+)"/);
    if (vForMatch) {
      props.push({ type: 'DIRECTIVE', name: 'for', exp: vForMatch[1] });
    }

    const vOnMatch = attrs.match(/@(\w+)="([^"]+)"/);
    if (vOnMatch) {
      props.push({ type: 'DIRECTIVE', name: 'on', arg: vOnMatch[1], exp: vOnMatch[2] });
    }

    const vBindMatch = attrs.match(/:(\w+)="([^"]+)"/);
    if (vBindMatch) {
      props.push({ type: 'DIRECTIVE', name: 'bind', arg: vBindMatch[1], exp: vBindMatch[2] });
    }

    const classMatch = attrs.match(/class="([^"]+)"/);
    if (classMatch) {
      props.push({ type: 'ATTRIBUTE', name: 'class', value: classMatch[1] });
    }

    // Parse children
    const childElements: object[] = [];
    const interpMatch = content.match(/\{\{\s*([^}]+)\s*\}\}/g);
    if (interpMatch) {
      interpMatch.forEach((interp) => {
        const exp = interp.match(/\{\{\s*([^}]+)\s*\}\}/)?.[1] || '';
        childElements.push({
          type: 'INTERPOLATION',
          content: { type: 'SIMPLE_EXPRESSION', content: exp.trim() },
        });
      });
    }

    children.push({
      type: 'ELEMENT',
      tag,
      tagType: 'ELEMENT',
      props,
      children: childElements,
      isSelfClosing: false,
    });
  }

  return {
    type: 'ROOT',
    children,
    helpers: [],
    components: [],
    directives: [],
    hoists: [],
    imports: [],
    cached: [],
  };
}

export function isWasmLoaded(): boolean {
  return wasmModule !== null && !usingMock;
}

export function isUsingMock(): boolean {
  return usingMock;
}

// Get the current WASM module (if loaded)
export function getWasm(): WasmModule | null {
  return wasmModule;
}
