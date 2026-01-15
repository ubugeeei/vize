<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted } from 'vue';
import MonacoEditor from './MonacoEditor.vue';
import * as monaco from 'monaco-editor';
import type { WasmModule, TypeCheckResult, TypeCheckDiagnostic, TypeCheckCapabilities } from '../wasm/index';

interface Diagnostic {
  message: string;
  startLine: number;
  startColumn: number;
  endLine?: number;
  endColumn?: number;
  severity: 'error' | 'warning' | 'info';
}

// Monaco TypeScript Worker diagnostics
interface TsDiagnostic {
  messageText: string | { messageText: string };
  message?: string;
  start: number;
  length: number;
  category: number; // 0=warning, 1=error, 2=suggestion, 3=message
  code: number;
}

// Source map entry for position mapping
interface SourceMapEntry {
  genStart: number;
  genEnd: number;
  srcStart: number;
  srcEnd: number;
}

const props = defineProps<{
  compiler: WasmModule | null;
}>();

const TYPECHECK_PRESET = `<script setup lang="ts">
import { ref } from 'vue'

// Props without type definition - triggers warning
const props = defineProps()

// Emits without type definition - triggers warning
const emit = defineEmits()

const count = ref(0)
const message = ref('Hello')

function increment() {
  count.value++
}
<\/script>

<template>
  <div class="container">
    <h1>{{ message }}</h1>
    <p>Count: {{ count }}</p>
    <button @click="increment">+1</button>
  </div>
</template>

<style scoped>
.container {
  padding: 20px;
}
</style>
`;

const TYPECHECK_TYPED_PRESET = `<script setup lang="ts">
import { ref } from 'vue'

// Props with type definition - no warning
interface Props {
  title: string
  count?: number
}
const props = defineProps<Props>()

// Emits with type definition - no warning
interface Emits {
  (e: 'update', value: number): void
  (e: 'reset'): void
}
const emit = defineEmits<Emits>()

const localCount = ref(props.count ?? 0)
const message = ref('Hello')

function increment() {
  localCount.value++
  emit('update', localCount.value)
}

function reset() {
  localCount.value = 0
  emit('reset')
}
<\/script>

<template>
  <div class="container">
    <h1>{{ props.title }}: {{ message }}</h1>
    <p>Count: {{ localCount }}</p>
    <button @click="increment">+1</button>
    <button @click="reset">Reset</button>
  </div>
</template>

<style scoped>
.container {
  padding: 20px;
}
button {
  margin: 0 4px;
}
</style>
`;

const source = ref(TYPECHECK_PRESET);
const typeCheckResult = ref<TypeCheckResult | null>(null);
const capabilities = ref<TypeCheckCapabilities | null>(null);
const error = ref<string | null>(null);
const activeTab = ref<'diagnostics' | 'virtualTs' | 'capabilities'>('diagnostics');
const checkTime = ref<number | null>(null);

// Options
const strictMode = ref(false);
const includeVirtualTs = ref(true);  // Enable by default to show Virtual TS
const checkProps = ref(true);
const checkEmits = ref(true);
const checkTemplateBindings = ref(true);

const STORAGE_KEY = 'vize-canon-typecheck-options';

// Use Monaco TypeScript for real type checking
const useMonacoTs = ref(true);
const tsDiagnostics = ref<Diagnostic[]>([]);
let virtualTsModel: monaco.editor.ITextModel | null = null;

// Configure Monaco TypeScript compiler options
async function configureTypeScript() {
  monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
    target: monaco.languages.typescript.ScriptTarget.ESNext,
    module: monaco.languages.typescript.ModuleKind.ESNext,
    moduleResolution: monaco.languages.typescript.ModuleResolutionKind.NodeJs,
    strict: strictMode.value,
    noEmit: true,
    allowJs: true,
    checkJs: false,
    esModuleInterop: true,
    skipLibCheck: true,
    jsx: monaco.languages.typescript.JsxEmit.Preserve,
    noImplicitAny: false,
    strictNullChecks: strictMode.value,
  });

  // Add Vue type declarations (module + compiler macros + globals)
  monaco.languages.typescript.typescriptDefaults.addExtraLib(VUE_GLOBALS_DECLARATIONS, 'vue.d.ts');
}

// Vue module and type declarations for Monaco TypeScript
const VUE_GLOBALS_DECLARATIONS = `
// Vue module declaration
declare module 'vue' {
  // Reactivity: Core
  export function ref<T>(value: T): Ref<T>;
  export function ref<T = any>(): Ref<T | undefined>;
  export function reactive<T extends object>(target: T): T;
  export function readonly<T extends object>(target: T): Readonly<T>;
  export function computed<T>(getter: () => T): ComputedRef<T>;
  export function computed<T>(options: { get: () => T; set: (value: T) => void }): WritableComputedRef<T>;

  // Reactivity: Utilities
  export function unref<T>(ref: T | Ref<T>): T;
  export function toRef<T extends object, K extends keyof T>(object: T, key: K): Ref<T[K]>;
  export function toRefs<T extends object>(object: T): { [K in keyof T]: Ref<T[K]> };
  export function isRef<T>(value: Ref<T> | unknown): value is Ref<T>;
  export function isReactive(value: unknown): boolean;
  export function isReadonly(value: unknown): boolean;
  export function isProxy(value: unknown): boolean;

  // Reactivity: Advanced
  export function shallowRef<T>(value: T): ShallowRef<T>;
  export function triggerRef(ref: ShallowRef): void;
  export function customRef<T>(factory: (track: () => void, trigger: () => void) => { get: () => T; set: (value: T) => void }): Ref<T>;
  export function toRaw<T>(observed: T): T;
  export function markRaw<T extends object>(value: T): T;

  // Lifecycle Hooks
  export function onMounted(callback: () => void): void;
  export function onUnmounted(callback: () => void): void;
  export function onBeforeMount(callback: () => void): void;
  export function onBeforeUnmount(callback: () => void): void;
  export function onUpdated(callback: () => void): void;
  export function onBeforeUpdate(callback: () => void): void;
  export function onActivated(callback: () => void): void;
  export function onDeactivated(callback: () => void): void;
  export function onErrorCaptured(callback: (err: unknown, instance: any, info: string) => boolean | void): void;

  // Watch
  export function watch<T>(source: () => T, callback: (newValue: T, oldValue: T) => void, options?: WatchOptions): () => void;
  export function watch<T>(source: Ref<T>, callback: (newValue: T, oldValue: T) => void, options?: WatchOptions): () => void;
  export function watchEffect(effect: () => void, options?: WatchOptions): () => void;

  // Dependency Injection
  export function provide<T>(key: string | symbol, value: T): void;
  export function inject<T>(key: string | symbol): T | undefined;
  export function inject<T>(key: string | symbol, defaultValue: T): T;

  // Misc
  export function nextTick(callback?: () => void): Promise<void>;
  export function getCurrentInstance(): any;

  // Types
  export interface Ref<T = any> {
    value: T;
  }
  export interface ComputedRef<T = any> extends Ref<T> {
    readonly value: T;
  }
  export interface WritableComputedRef<T> extends Ref<T> {}
  export interface ShallowRef<T = any> extends Ref<T> {}
  export type UnwrapRef<T> = T extends Ref<infer V> ? V : T;
  export type Reactive<T> = T;
  export type MaybeRef<T> = T | Ref<T>;

  export interface WatchOptions {
    immediate?: boolean;
    deep?: boolean;
    flush?: 'pre' | 'post' | 'sync';
  }
}

// Vue Compiler Macros (available in <script setup>)
declare function defineProps<T>(): Readonly<T>;
declare function defineEmits<T>(): T;
declare function defineExpose<T>(exposed?: T): void;
declare function defineOptions<T>(options: T): void;
declare function defineSlots<T>(): T;
declare function defineModel<T>(name?: string, options?: { required?: boolean; default?: T }): import('vue').Ref<T>;
declare function withDefaults<T, D extends Partial<T>>(props: T, defaults: D): T & D;

// Vue Global Instance Properties (available in templates)
declare const $attrs: Record<string, unknown>;
declare const $slots: Record<string, (...args: any[]) => any>;
declare const $refs: Record<string, any>;
declare const $el: HTMLElement | undefined;
declare const $parent: any;
declare const $root: any;
declare const $emit: (...args: any[]) => void;
declare const $forceUpdate: () => void;
declare const $nextTick: (callback?: () => void) => Promise<void>;

// Event handler context
declare const $event: Event;
`;

// Cached source map entries for hover
let cachedSourceMap: SourceMapEntry[] = [];
let cachedVirtualTs: string = '';

// Virtual TS model URI - use ts-nul-authority scheme for Monaco TypeScript worker
const VIRTUAL_TS_URI = monaco.Uri.parse('ts:virtual-sfc.ts');

// Get hover info from TypeScript at a given position in Virtual TS
async function getTypeScriptHover(genOffset: number): Promise<string | null> {
  if (!virtualTsModel) return null;

  try {
    const worker = await monaco.languages.typescript.getTypeScriptWorker();
    const client = await worker(VIRTUAL_TS_URI);

    // Get quick info at position
    const quickInfo = await client.getQuickInfoAtPosition(VIRTUAL_TS_URI.toString(), genOffset);
    if (!quickInfo) return null;

    // Build hover content
    const parts: string[] = [];

    if (quickInfo.displayParts) {
      const displayText = quickInfo.displayParts.map((p: { text: string }) => p.text).join('');
      if (displayText) {
        parts.push('```typescript\n' + displayText + '\n```');
      }
    }

    if (quickInfo.documentation) {
      const docs = quickInfo.documentation.map((d: { text: string }) => d.text).join('\n');
      if (docs) {
        parts.push(docs);
      }
    }

    return parts.length > 0 ? parts.join('\n\n') : null;
  } catch (e) {
    console.error('Failed to get TypeScript hover:', e);
    return null;
  }
}

// Map source offset to generated offset using source map
function mapSourceToGenerated(srcOffset: number): number | null {
  for (const entry of cachedSourceMap) {
    if (srcOffset >= entry.srcStart && srcOffset < entry.srcEnd) {
      // Calculate relative position within the source range
      const relativeOffset = srcOffset - entry.srcStart;
      return entry.genStart + relativeOffset;
    }
  }
  return null;
}

// Register hover provider for Vue language
let hoverProviderDisposable: monaco.IDisposable | null = null;

function registerHoverProvider() {
  if (hoverProviderDisposable) {
    hoverProviderDisposable.dispose();
  }

  hoverProviderDisposable = monaco.languages.registerHoverProvider('vue', {
    async provideHover(model, position) {
      // Convert position to offset
      const srcOffset = model.getOffsetAt(position);

      // Map to Virtual TS offset
      const genOffset = mapSourceToGenerated(srcOffset);
      if (genOffset === null) return null;

      // Get hover info from TypeScript
      const hoverContent = await getTypeScriptHover(genOffset);
      if (!hoverContent) return null;

      // Return hover info
      return {
        contents: [{ value: hoverContent }],
      };
    },
  });
}

// Get TypeScript diagnostics from Monaco Worker
async function getTypeScriptDiagnostics(virtualTs: string): Promise<Diagnostic[]> {
  if (!virtualTs) return [];

  // Create or update the virtual TS model
  if (virtualTsModel) {
    virtualTsModel.setValue(virtualTs);
  } else {
    virtualTsModel = monaco.editor.createModel(virtualTs, 'typescript', VIRTUAL_TS_URI);
  }

  try {
    // Get TypeScript Worker
    const worker = await monaco.languages.typescript.getTypeScriptWorker();
    const client = await worker(VIRTUAL_TS_URI);

    // Get semantic and syntactic diagnostics
    const [semanticDiags, syntacticDiags] = await Promise.all([
      client.getSemanticDiagnostics(VIRTUAL_TS_URI.toString()),
      client.getSyntacticDiagnostics(VIRTUAL_TS_URI.toString()),
    ]);

    const allDiags = [...syntacticDiags, ...semanticDiags] as TsDiagnostic[];

    console.log('[TypeCheck] Virtual TS diagnostics:', allDiags.length, JSON.stringify(allDiags, null, 2));

    // Convert to our Diagnostic format
    return allDiags.map(d => {
      const startPos = virtualTsModel!.getPositionAt(d.start);
      const endPos = virtualTsModel!.getPositionAt(d.start + d.length);

      // Extract message text - can be string, object with messageText, or nested chain
      let message = 'Unknown error';
      if (typeof d.messageText === 'string') {
        message = d.messageText;
      } else if (d.messageText && typeof d.messageText === 'object') {
        // DiagnosticMessageChain - get the first message
        message = (d.messageText as any).messageText || 'Unknown error';
      } else if (typeof d.message === 'string') {
        message = d.message;
      }

      // TypeScript DiagnosticCategory: 0=Warning, 1=Error, 2=Suggestion, 3=Message
      const severity = d.category === 1 ? 'error' : d.category === 0 ? 'warning' : 'info';

      return {
        message,
        startLine: startPos.lineNumber,
        startColumn: startPos.column,
        endLine: endPos.lineNumber,
        endColumn: endPos.column,
        severity: severity as 'error' | 'warning' | 'info',
      };
    });
  } catch (e) {
    console.error('Failed to get TypeScript diagnostics:', e);
    return [];
  }
}

// Parse source map from generated Virtual TS
function parseSourceMap(virtualTs: string): SourceMapEntry[] {
  const entries: SourceMapEntry[] = [];

  // Look for source map markers in comments
  // Format: // @vize-map: genStart:genEnd -> srcStart:srcEnd
  const regex = /\/\/ @vize-map:\s*(\d+):(\d+)\s*->\s*(\d+):(\d+)/g;
  let match;
  while ((match = regex.exec(virtualTs)) !== null) {
    entries.push({
      genStart: parseInt(match[1]),
      genEnd: parseInt(match[2]),
      srcStart: parseInt(match[3]),
      srcEnd: parseInt(match[4]),
    });
  }

  return entries;
}

// Map diagnostics from Virtual TS to original Vue source
function mapDiagnosticsToSource(
  tsDiags: Diagnostic[],
  virtualTs: string,
  vueSource: string
): Diagnostic[] {
  // Parse source map entries from Virtual TS comments
  const sourceMapEntries = parseSourceMap(virtualTs);

  const mapped: Diagnostic[] = [];

  // Helper: convert line/column to offset
  function lineColToOffset(content: string, line: number, col: number): number {
    const lines = content.split('\n');
    let offset = 0;
    for (let i = 0; i < line - 1 && i < lines.length; i++) {
      offset += lines[i].length + 1; // +1 for newline
    }
    return offset + col - 1;
  }

  // Helper: convert offset to line/column in Vue source
  function offsetToLineCol(content: string, offset: number): { line: number; col: number } {
    const lines = content.split('\n');
    let currentOffset = 0;
    for (let i = 0; i < lines.length; i++) {
      const lineEnd = currentOffset + lines[i].length + 1;
      if (offset < lineEnd) {
        return { line: i + 1, col: offset - currentOffset + 1 };
      }
      currentOffset = lineEnd;
    }
    return { line: lines.length, col: 1 };
  }

  for (const diag of tsDiags) {
    // Calculate offset in virtual TS
    const diagOffset = lineColToOffset(virtualTs, diag.startLine, diag.startColumn);
    const diagEndOffset = lineColToOffset(virtualTs, diag.endLine || diag.startLine, diag.endColumn || diag.startColumn);

    // Try to find a matching source map entry
    let foundMapping = false;
    for (const entry of sourceMapEntries) {
      if (diagOffset >= entry.genStart && diagOffset <= entry.genEnd) {
        // Calculate relative position within the generated range
        const relativeOffset = diagOffset - entry.genStart;
        const srcOffset = entry.srcStart + relativeOffset;
        const srcEndOffset = Math.min(entry.srcEnd, srcOffset + (diagEndOffset - diagOffset));

        const startPos = offsetToLineCol(vueSource, srcOffset);
        const endPos = offsetToLineCol(vueSource, srcEndOffset);

        mapped.push({
          ...diag,
          startLine: startPos.line,
          startColumn: startPos.col,
          endLine: endPos.line,
          endColumn: endPos.col,
          message: `[TS] ${diag.message}`,
        });
        foundMapping = true;
        break;
      }
    }

    // Only include diagnostics that have valid source mappings
    // Skip diagnostics from Virtual TS boilerplate (no mapping = generated code)
    if (!foundMapping) {
      // Skip errors from boilerplate - these are not user code errors
      console.log('[TypeCheck] Skipping unmapped diagnostic:', diag.message);
    }
  }

  return mapped;
}

// Load saved options from localStorage
function loadOptions() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      const config = JSON.parse(saved);
      strictMode.value = config.strictMode ?? false;
      includeVirtualTs.value = config.includeVirtualTs ?? true;  // Default to true
      checkProps.value = config.checkProps ?? true;
      checkEmits.value = config.checkEmits ?? true;
      checkTemplateBindings.value = config.checkTemplateBindings ?? true;
      useMonacoTs.value = config.useMonacoTs ?? true; // Default to true
    }
  } catch (e) {
    console.warn('Failed to load options:', e);
  }
}

// Save options to localStorage
function saveOptions() {
  try {
    const config = {
      strictMode: strictMode.value,
      includeVirtualTs: includeVirtualTs.value,
      checkProps: checkProps.value,
      checkEmits: checkEmits.value,
      checkTemplateBindings: checkTemplateBindings.value,
      useMonacoTs: useMonacoTs.value,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
  } catch (e) {
    console.warn('Failed to save options:', e);
  }
}

const errorCount = computed(() => {
  const wasmErrors = typeCheckResult.value?.errorCount ?? 0;
  const tsErrors = tsDiagnostics.value.filter(d => d.severity === 'error').length;
  return wasmErrors + tsErrors;
});
const warningCount = computed(() => {
  const wasmWarnings = typeCheckResult.value?.warningCount ?? 0;
  const tsWarnings = tsDiagnostics.value.filter(d => d.severity === 'warning').length;
  return wasmWarnings + tsWarnings;
});

// Calculate position from offset
function getPositionFromOffset(source: string, offset: number): { line: number; column: number } {
  const lines = source.substring(0, offset).split('\n');
  return {
    line: lines.length,
    column: lines[lines.length - 1].length + 1,
  };
}

// Convert type check diagnostics to Monaco markers (combining WASM and TS Worker diagnostics)
const diagnostics = computed((): Diagnostic[] => {
  const wasmDiags: Diagnostic[] = [];

  // Add WASM-based diagnostics
  if (typeCheckResult.value?.diagnostics) {
    for (const d of typeCheckResult.value.diagnostics) {
      const startPos = getPositionFromOffset(source.value, d.start);
      const endPos = getPositionFromOffset(source.value, d.end);
      const message = d.code ? `[${d.code}] ${d.message}` : d.message;
      wasmDiags.push({
        message,
        startLine: startPos.line,
        startColumn: startPos.column,
        endLine: endPos.line,
        endColumn: endPos.column,
        severity: d.severity === 'error' ? 'error' : d.severity === 'warning' ? 'warning' : 'info',
      });
    }
  }

  // Add TypeScript Worker diagnostics
  if (useMonacoTs.value) {
    return [...wasmDiags, ...tsDiagnostics.value];
  }

  return wasmDiags;
});

async function typeCheck() {
  if (!props.compiler) return;

  const startTime = performance.now();
  error.value = null;

  try {
    const result = props.compiler.typeCheck(source.value, {
      filename: 'example.vue',
      strict: strictMode.value,
      includeVirtualTs: true, // Always generate virtual TS for Monaco checking
      checkProps: checkProps.value,
      checkEmits: checkEmits.value,
      checkTemplateBindings: checkTemplateBindings.value,
    });
    typeCheckResult.value = result;

    // If Monaco TS checking is enabled and we have virtual TS
    if (useMonacoTs.value && result.virtualTs) {
      // Cache source map for hover
      cachedVirtualTs = result.virtualTs;
      cachedSourceMap = parseSourceMap(result.virtualTs);

      const tsDiags = await getTypeScriptDiagnostics(result.virtualTs);
      tsDiagnostics.value = mapDiagnosticsToSource(tsDiags, result.virtualTs, source.value);
    } else {
      tsDiagnostics.value = [];
      cachedSourceMap = [];
      cachedVirtualTs = '';
    }

    checkTime.value = performance.now() - startTime;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    typeCheckResult.value = null;
    tsDiagnostics.value = [];
  }
}

function loadCapabilities() {
  if (!props.compiler) return;

  try {
    capabilities.value = props.compiler.getTypeCheckCapabilities();
  } catch (e) {
    console.error('Failed to load capabilities:', e);
  }
}

function getSeverityIcon(severity: 'error' | 'warning' | 'info' | 'hint'): string {
  switch (severity) {
    case 'error': return '\u2717';
    case 'warning': return '\u26A0';
    case 'info': return '\u24D8';
    default: return '\u2022';
  }
}

function setPreset(preset: 'untyped' | 'typed') {
  source.value = preset === 'typed' ? TYPECHECK_TYPED_PRESET : TYPECHECK_PRESET;
}

let checkTimer: ReturnType<typeof setTimeout> | null = null;

watch(
  source,
  () => {
    if (checkTimer) clearTimeout(checkTimer);
    checkTimer = setTimeout(typeCheck, 300);
  },
  { immediate: true }
);

watch(
  [strictMode, includeVirtualTs, checkProps, checkEmits, checkTemplateBindings, useMonacoTs],
  () => {
    saveOptions();
    typeCheck();
  }
);

watch(
  () => props.compiler,
  () => {
    if (props.compiler) {
      typeCheck();
      loadCapabilities();
    }
  }
);

onMounted(async () => {
  loadOptions();
  await configureTypeScript();
  registerHoverProvider();
  if (props.compiler) {
    loadCapabilities();
  }
});

onUnmounted(() => {
  // Clean up the virtual TS model
  if (virtualTsModel) {
    virtualTsModel.dispose();
    virtualTsModel = null;
  }
  // Clean up hover provider
  if (hoverProviderDisposable) {
    hoverProviderDisposable.dispose();
    hoverProviderDisposable = null;
  }
});
</script>

<template>
  <div class="typecheck-playground">
    <div class="panel input-panel">
      <div class="panel-header">
        <div class="header-title">
          <span class="icon">&lt;/&gt;</span>
          <h2>Source</h2>
        </div>
        <div class="panel-actions">
          <button @click="setPreset('untyped')" class="btn-ghost">Untyped</button>
          <button @click="setPreset('typed')" class="btn-ghost">Typed</button>
        </div>
      </div>
      <div class="editor-container">
        <MonacoEditor v-model="source" language="vue" :diagnostics="diagnostics" />
      </div>
    </div>

    <div class="panel output-panel">
      <div class="panel-header">
        <div class="header-title">
          <span class="icon">&#x2714;</span>
          <h2>Type Analysis</h2>
          <span v-if="checkTime !== null" class="perf-badge">
            {{ checkTime.toFixed(2) }}ms
          </span>
          <template v-if="typeCheckResult">
            <span v-if="errorCount > 0" class="count-badge errors">{{ errorCount }}</span>
            <span v-if="warningCount > 0" class="count-badge warnings">{{ warningCount }}</span>
          </template>
        </div>
        <div class="tabs">
          <button
            :class="['tab', { active: activeTab === 'diagnostics' }]"
            @click="activeTab = 'diagnostics'"
          >Diagnostics
            <span v-if="diagnostics.length" class="tab-badge">{{ diagnostics.length }}</span>
          </button>
          <button
            :class="['tab', { active: activeTab === 'virtualTs' }]"
            @click="activeTab = 'virtualTs'"
          >Virtual TS</button>
          <button
            :class="['tab', { active: activeTab === 'capabilities' }]"
            @click="activeTab = 'capabilities'"
          >Info</button>
        </div>
      </div>

      <div class="output-content">
        <div v-if="error" class="error-panel">
          <div class="error-header">Type Check Error</div>
          <pre class="error-content">{{ error }}</pre>
        </div>

        <template v-else-if="typeCheckResult">
          <!-- Diagnostics Tab -->
          <div v-if="activeTab === 'diagnostics'" class="diagnostics-output">
            <div class="output-header-bar">
              <span class="output-title">Type Issues</span>
              <div class="options-toggle">
                <label class="option-label">
                  <input type="checkbox" v-model="strictMode" />
                  Strict
                </label>
              </div>
            </div>

            <div class="options-panel">
              <label class="option-label highlight">
                <input type="checkbox" v-model="useMonacoTs" />
                TypeScript (Monaco)
              </label>
              <label class="option-label">
                <input type="checkbox" v-model="checkProps" />
                Check Props
              </label>
              <label class="option-label">
                <input type="checkbox" v-model="checkEmits" />
                Check Emits
              </label>
              <label class="option-label">
                <input type="checkbox" v-model="checkTemplateBindings" />
                Check Template Bindings
              </label>
              <label class="option-label">
                <input type="checkbox" v-model="includeVirtualTs" />
                Show Virtual TS
              </label>
            </div>

            <div v-if="diagnostics.length === 0" class="success-state">
              <span class="success-icon">&#x2713;</span>
              <span>No type issues found</span>
            </div>

            <div v-else class="diagnostics-list">
              <div
                v-for="(diagnostic, i) in diagnostics"
                :key="i"
                :class="['diagnostic-item', `severity-${diagnostic.severity}`]"
              >
                <div class="diagnostic-header">
                  <span class="severity-icon">{{ getSeverityIcon(diagnostic.severity) }}</span>
                  <span class="location-badge">
                    {{ diagnostic.startLine }}:{{ diagnostic.startColumn }}
                  </span>
                </div>
                <div class="diagnostic-message">{{ diagnostic.message }}</div>
              </div>
            </div>
          </div>

          <!-- Virtual TS Tab -->
          <div v-else-if="activeTab === 'virtualTs'" class="virtualts-output">
            <div class="output-header-bar">
              <span class="output-title">Generated TypeScript</span>
            </div>
            <div v-if="typeCheckResult.virtualTs" class="editor-container">
              <MonacoEditor
                :model-value="typeCheckResult.virtualTs"
                language="typescript"
                :read-only="true"
              />
            </div>
            <div v-else class="empty-state">
              <span>Enable "Generate Virtual TS" option to see generated TypeScript</span>
            </div>
          </div>

          <!-- Capabilities Tab -->
          <div v-else-if="activeTab === 'capabilities'" class="capabilities-output">
            <div class="output-header-bar">
              <span class="output-title">Type Checker Capabilities</span>
            </div>

            <div v-if="capabilities" class="capabilities-content">
              <div class="capability-section">
                <h3>Mode</h3>
                <code class="mode-badge">{{ capabilities.mode }}</code>
                <p>{{ capabilities.description }}</p>
              </div>

              <div class="capability-section">
                <h3>Available Checks</h3>
                <div class="checks-list">
                  <div v-for="check in capabilities.checks" :key="check.name" class="check-item">
                    <code class="check-name">{{ check.name }}</code>
                    <span :class="['check-severity', check.severity]">{{ check.severity }}</span>
                    <p class="check-description">{{ check.description }}</p>
                  </div>
                </div>
              </div>

              <div class="capability-section">
                <h3>Notes</h3>
                <ul class="notes-list">
                  <li v-for="(note, i) in capabilities.notes" :key="i">{{ note }}</li>
                </ul>
              </div>
            </div>
          </div>
        </template>

        <div v-else class="loading-state">
          <span>Enter Vue code to see type analysis</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.typecheck-playground {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0;
  height: 100%;
  min-height: 0;
  grid-column: 1 / -1;
  background: var(--bg-primary);
}

.panel {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
}

.input-panel {
  border-right: 1px solid var(--border-primary);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-primary);
  flex-shrink: 0;
}

.header-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.header-title .icon {
  font-size: 1rem;
  color: var(--accent-blue);
}

.header-title h2 {
  font-size: 0.875rem;
  font-weight: 600;
  margin: 0;
}

.perf-badge {
  font-size: 0.625rem;
  padding: 0.125rem 0.375rem;
  background: rgba(74, 222, 128, 0.15);
  color: #4ade80;
  border-radius: 3px;
  font-family: 'JetBrains Mono', monospace;
}

.count-badge {
  font-size: 0.625rem;
  padding: 0.0625rem 0.375rem;
  border-radius: 8px;
  min-width: 1.25rem;
  text-align: center;
  font-family: 'JetBrains Mono', monospace;
}

.count-badge.errors {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
}

.count-badge.warnings {
  background: rgba(245, 158, 11, 0.2);
  color: #fbbf24;
}

.panel-actions {
  display: flex;
  gap: 0.5rem;
}

.btn-ghost {
  padding: 0.25rem 0.5rem;
  font-size: 0.75rem;
  background: transparent;
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s;
}

.btn-ghost:hover {
  background: var(--bg-tertiary);
  color: var(--text-primary);
}

.tabs {
  display: flex;
  gap: 0.125rem;
}

.tab {
  padding: 0.375rem 0.625rem;
  font-size: 0.75rem;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s;
  display: flex;
  align-items: center;
  gap: 0.375rem;
}

.tab:hover {
  color: var(--text-secondary);
  background: var(--bg-tertiary);
}

.tab.active {
  color: var(--text-primary);
  background: var(--bg-tertiary);
  font-weight: 500;
}

.tab-badge {
  font-size: 0.625rem;
  padding: 0.0625rem 0.3125rem;
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
  border-radius: 8px;
  min-width: 1rem;
  text-align: center;
}

.editor-container {
  flex: 1;
  overflow: hidden;
}

.output-content {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
}

/* Error State */
.error-panel {
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
  border-radius: 6px;
  overflow: hidden;
}

.error-header {
  padding: 0.5rem 0.75rem;
  background: rgba(239, 68, 68, 0.15);
  color: #f87171;
  font-size: 0.75rem;
  font-weight: 600;
}

.error-content {
  padding: 0.75rem;
  font-size: 0.75rem;
  color: #fca5a5;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
}

/* Output Header Bar */
.output-header-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.15), rgba(139, 92, 246, 0.15));
  border: 1px solid rgba(59, 130, 246, 0.3);
  border-radius: 4px;
  margin-bottom: 0.75rem;
}

.output-title {
  font-size: 0.75rem;
  font-weight: 600;
  color: #60a5fa;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

/* Options */
.options-panel {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  padding: 0.5rem 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  margin-bottom: 0.75rem;
}

.option-label {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  font-size: 0.6875rem;
  color: var(--text-secondary);
  cursor: pointer;
}

.option-label input[type="checkbox"] {
  width: 12px;
  height: 12px;
  accent-color: var(--accent-blue);
}

.option-label.highlight {
  background: rgba(59, 130, 246, 0.15);
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  border: 1px solid rgba(59, 130, 246, 0.3);
}

/* Success State */
.success-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 2rem;
  color: #4ade80;
  font-size: 0.875rem;
}

.success-icon {
  font-size: 1.25rem;
}

/* Diagnostics List */
.diagnostics-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.diagnostic-item {
  padding: 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  border-left: 3px solid;
}

.diagnostic-item.severity-error {
  border-left-color: #ef4444;
}

.diagnostic-item.severity-warning {
  border-left-color: #f59e0b;
}

.diagnostic-item.severity-info {
  border-left-color: #60a5fa;
}

.diagnostic-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.375rem;
}

.severity-icon {
  font-size: 0.75rem;
  font-weight: bold;
}

.severity-error .severity-icon {
  color: #ef4444;
}

.severity-warning .severity-icon {
  color: #f59e0b;
}

.severity-info .severity-icon {
  color: #60a5fa;
}

.diagnostic-code {
  font-size: 0.6875rem;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-muted);
  background: var(--bg-tertiary);
  padding: 0.125rem 0.375rem;
  border-radius: 3px;
}

.location-badge {
  margin-left: auto;
  font-size: 0.625rem;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-muted);
}

.diagnostic-message {
  font-size: 0.8125rem;
  color: var(--text-primary);
  line-height: 1.4;
}

.diagnostic-help {
  display: flex;
  align-items: flex-start;
  gap: 0.375rem;
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-top: 0.5rem;
  padding: 0.5rem;
  background: var(--bg-tertiary);
  border-radius: 4px;
}

.help-label {
  color: #4ade80;
  font-weight: 500;
  flex-shrink: 0;
}

/* Virtual TS Output */
.virtualts-output {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.virtualts-output .editor-container {
  flex: 1;
  min-height: 200px;
}

/* Capabilities Output */
.capabilities-content {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.capability-section h3 {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-secondary);
  margin: 0 0 0.5rem 0;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.capability-section p {
  font-size: 0.8125rem;
  color: var(--text-muted);
  margin: 0.25rem 0 0 0;
}

.mode-badge {
  display: inline-block;
  padding: 0.25rem 0.5rem;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  font-size: 0.75rem;
  color: #60a5fa;
}

.checks-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.check-item {
  padding: 0.5rem 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
}

.check-name {
  font-size: 0.75rem;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-primary);
}

.check-severity {
  margin-left: 0.5rem;
  font-size: 0.625rem;
  padding: 0.125rem 0.375rem;
  border-radius: 3px;
  text-transform: uppercase;
}

.check-severity.error {
  background: rgba(239, 68, 68, 0.15);
  color: #f87171;
}

.check-severity.warning {
  background: rgba(245, 158, 11, 0.15);
  color: #fbbf24;
}

.check-description {
  font-size: 0.6875rem;
  color: var(--text-muted);
  margin: 0.25rem 0 0 0;
}

.notes-list {
  margin: 0;
  padding-left: 1rem;
  color: var(--text-muted);
  font-size: 0.75rem;
}

.notes-list li {
  margin: 0.25rem 0;
}

.empty-state,
.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  color: var(--text-muted);
  font-size: 0.875rem;
}

/* Mobile responsive */
@media (max-width: 768px) {
  .typecheck-playground {
    grid-template-columns: 1fr;
    grid-template-rows: minmax(300px, 1fr) minmax(300px, 1fr);
    height: auto;
    min-height: 100%;
  }

  .panel {
    min-height: 300px;
  }

  .input-panel {
    border-right: none;
    border-bottom: 1px solid var(--border-primary);
  }

  .panel-header {
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .tabs {
    flex-wrap: wrap;
    width: 100%;
  }

  .options-panel {
    flex-direction: column;
  }
}
</style>
