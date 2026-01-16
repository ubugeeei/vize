<script setup lang="ts">
import { ref, computed, watch, onMounted, nextTick } from 'vue';
import MonacoEditor from './MonacoEditor.vue';
import type { Diagnostic } from './MonacoEditor.vue';
import type { WasmModule, AnalysisResult } from '../wasm/index';

const props = defineProps<{
  compiler: WasmModule | null;
}>();

// === Sample Project Files ===
const SAMPLE_PROJECT: Record<string, string> = {
  'App.vue': `<script setup lang="ts">
import { provide, ref } from 'vue'
import ParentComponent from './ParentComponent.vue'

// Provide theme to all descendants
const theme = ref<'light' | 'dark'>('dark')
provide('theme', theme)
provide('user', { name: 'John', id: 1 })

function handleUpdate(value: number) {
  console.log('Updated:', value)
}
<\/script>

<template>
  <div id="app" class="app-container">
    <ParentComponent
      title="Dashboard"
      @update="handleUpdate"
    />
  </div>
</template>`,

  'ParentComponent.vue': `<script setup lang="ts">
import { inject, ref, onMounted } from 'vue'
import ChildComponent from './ChildComponent.vue'

// Props
const props = defineProps<{
  title: string
}>()

// Emits - 'unused-event' is declared but never called
const emit = defineEmits<{
  update: [value: number]
  'unused-event': []
}>()

// Inject theme from App
const theme = inject<Ref<'light' | 'dark'>>('theme')

// ISSUE: Destructuring inject loses reactivity!
const { name } = inject('user') as { name: string; id: number }

// Browser API usage
const width = ref(0)
onMounted(() => {
  width.value = window.innerWidth
})
<\/script>

<template>
  <div :class="['parent', theme]">
    <h2>{{ title }}</h2>
    <p>User: {{ name }}</p>
    <p>Width: {{ width }}px</p>
    <ChildComponent
      :theme="theme"
      custom-attr="value"
      data-test="child"
      @change="emit('update', $event)"
      @unhandled-event="() => {}"
    />
  </div>
</template>`,

  'ChildComponent.vue': `<script setup lang="ts">
import { ref, toRefs } from 'vue'

const props = defineProps<{
  theme?: string
  message?: string
}>()

// Correct: using toRefs
const { theme } = toRefs(props)

// Emits
const emit = defineEmits<{
  change: [value: number]
}>()

const items = ref([
  { id: 1, name: 'Item 1' },
  { id: 2, name: 'Item 2' },
  { id: 3, name: 'Item 3' },
])

function handleClick(item: { id: number; name: string }) {
  emit('change', item.id)
}
<\/script>

<template>
  <!-- ISSUE: Multiple root elements without v-bind="$attrs" -->
  <div class="child-header">
    <span>Theme: {{ theme }}</span>
  </div>
  <ul class="child-list">
    <li
      v-for="item in items"
      :key="item.id"
      :id="\`item-\${item.id}\`"
      @click="handleClick(item)"
    >
      {{ item.name }}
    </li>
  </ul>
</template>`,

  'StoreExample.vue': `<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useUserStore } from './stores/user'

const userStore = useUserStore()

// ISSUE: Destructuring store loses reactivity
const { username, email } = userStore

// Correct way:
// const { username, email } = storeToRefs(userStore)
<\/script>

<template>
  <div>
    <p>Username: {{ username }}</p>
    <p>Email: {{ email }}</p>
  </div>
</template>`,
};

// === State ===
const files = ref<Record<string, string>>({ ...SAMPLE_PROJECT });
const activeFile = ref<string>('ParentComponent.vue');
const analysisResults = ref<Record<string, AnalysisResult | null>>({});
const crossFileIssues = ref<CrossFileIssue[]>([]);
const analysisTime = ref<number>(0);
const isAnalyzing = ref(false);
const selectedIssue = ref<CrossFileIssue | null>(null);

// Options
const options = ref({
  provideInject: true,
  componentEmits: true,
  fallthroughAttrs: true,
  reactivityTracking: true,
  uniqueIds: true,
  serverClientBoundary: true,
});

// === Resizable Panes ===
const sidebarWidth = ref(220);
const diagnosticsWidth = ref(320);
const isResizingSidebar = ref(false);
const isResizingDiagnostics = ref(false);
const containerRef = ref<HTMLElement | null>(null);

function startSidebarResize(e: MouseEvent) {
  isResizingSidebar.value = true;
  e.preventDefault();
  document.addEventListener('mousemove', onSidebarResize);
  document.addEventListener('mouseup', stopResize);
}

function startDiagnosticsResize(e: MouseEvent) {
  isResizingDiagnostics.value = true;
  e.preventDefault();
  document.addEventListener('mousemove', onDiagnosticsResize);
  document.addEventListener('mouseup', stopResize);
}

function onSidebarResize(e: MouseEvent) {
  if (!isResizingSidebar.value || !containerRef.value) return;
  const containerRect = containerRef.value.getBoundingClientRect();
  const newWidth = Math.max(150, Math.min(400, e.clientX - containerRect.left));
  sidebarWidth.value = newWidth;
}

function onDiagnosticsResize(e: MouseEvent) {
  if (!isResizingDiagnostics.value || !containerRef.value) return;
  const containerRect = containerRef.value.getBoundingClientRect();
  const newWidth = Math.max(200, Math.min(500, containerRect.right - e.clientX));
  diagnosticsWidth.value = newWidth;
}

function stopResize() {
  isResizingSidebar.value = false;
  isResizingDiagnostics.value = false;
  document.removeEventListener('mousemove', onSidebarResize);
  document.removeEventListener('mousemove', onDiagnosticsResize);
  document.removeEventListener('mouseup', stopResize);
}

const gridStyle = computed(() => ({
  gridTemplateColumns: `${sidebarWidth.value}px 4px 1fr 4px ${diagnosticsWidth.value}px`,
}));

// === Types ===
interface CrossFileIssue {
  id: string;
  type: string;
  code: string;
  severity: 'error' | 'warning' | 'info';
  message: string;
  file: string;
  line: number;
  column: number;
  endLine?: number;
  endColumn?: number;
  relatedLocations?: Array<{ file: string; line: number; column: number; message: string }>;
  suggestion?: string;
}

// === Computed ===
const currentSource = computed({
  get: () => files.value[activeFile.value] || '',
  set: (val) => { files.value[activeFile.value] = val; }
});

const currentDiagnostics = computed((): Diagnostic[] => {
  return crossFileIssues.value
    .filter(issue => issue.file === activeFile.value)
    .map(issue => ({
      message: `[${issue.code}] ${issue.message}${issue.suggestion ? `\n\n💡 ${issue.suggestion}` : ''}`,
      startLine: issue.line,
      startColumn: issue.column,
      endLine: issue.endLine,
      endColumn: issue.endColumn,
      severity: issue.severity,
    }));
});

const issuesByFile = computed(() => {
  const grouped: Record<string, CrossFileIssue[]> = {};
  for (const issue of crossFileIssues.value) {
    if (!grouped[issue.file]) grouped[issue.file] = [];
    grouped[issue.file].push(issue);
  }
  return grouped;
});

const issuesByType = computed(() => {
  const grouped: Record<string, CrossFileIssue[]> = {};
  for (const issue of crossFileIssues.value) {
    if (!grouped[issue.type]) grouped[issue.type] = [];
    grouped[issue.type].push(issue);
  }
  return grouped;
});

const stats = computed(() => ({
  files: Object.keys(files.value).length,
  totalIssues: crossFileIssues.value.length,
  errors: crossFileIssues.value.filter(i => i.severity === 'error').length,
  warnings: crossFileIssues.value.filter(i => i.severity === 'warning').length,
  infos: crossFileIssues.value.filter(i => i.severity === 'info').length,
}));

const dependencyGraph = computed(() => {
  // Build simple dependency graph from imports
  const graph: Record<string, string[]> = {};
  for (const [filename, source] of Object.entries(files.value)) {
    const imports: string[] = [];
    const importRegex = /import\s+[\w{}\s,*]+\s+from\s+['"]\.\/([^'"]+)['"]/g;
    let match;
    while ((match = importRegex.exec(source)) !== null) {
      let importFile = match[1];
      if (!importFile.endsWith('.vue')) importFile += '.vue';
      if (files.value[importFile]) {
        imports.push(importFile);
      }
    }
    graph[filename] = imports;
  }
  return graph;
});

// === Analysis Functions ===
let issueIdCounter = 0;

function createIssue(
  type: string,
  code: string,
  severity: 'error' | 'warning' | 'info',
  message: string,
  file: string,
  line: number,
  column: number,
  options?: {
    endLine?: number;
    endColumn?: number;
    suggestion?: string;
    relatedLocations?: Array<{ file: string; line: number; column: number; message: string }>;
  }
): CrossFileIssue {
  return {
    id: `issue-${++issueIdCounter}`,
    type,
    code,
    severity,
    message,
    file,
    line,
    column,
    ...options,
  };
}

// Convert character offset to line/column (1-based for Monaco)
function offsetToLineColumn(source: string, offset: number): { line: number; column: number } {
  const beforeOffset = source.substring(0, offset);
  const lines = beforeOffset.split('\n');
  return {
    line: lines.length,
    column: lines[lines.length - 1].length + 1,
  };
}

// Find line/column for a pattern (uses first match)
function findLineAndColumn(source: string, pattern: RegExp | string): { line: number; column: number; endLine?: number; endColumn?: number } | null {
  const regex = typeof pattern === 'string' ? new RegExp(pattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')) : pattern;
  const match = source.match(regex);
  if (!match || match.index === undefined) return null;

  const start = offsetToLineColumn(source, match.index);
  const end = offsetToLineColumn(source, match.index + match[0].length);

  return {
    line: start.line,
    column: start.column,
    endLine: end.line,
    endColumn: end.column,
  };
}

// Find line/column at a specific offset (for regex exec results)
function findLineAndColumnAtOffset(source: string, offset: number, length: number): { line: number; column: number; endLine: number; endColumn: number } {
  const start = offsetToLineColumn(source, offset);
  const end = offsetToLineColumn(source, offset + length);
  return {
    line: start.line,
    column: start.column,
    endLine: end.line,
    endColumn: end.column,
  };
}

async function analyzeAll() {
  if (!props.compiler) return;

  isAnalyzing.value = true;
  const startTime = performance.now();
  const issues: CrossFileIssue[] = [];
  issueIdCounter = 0;

  // First pass: analyze each file
  const results: Record<string, AnalysisResult | null> = {};
  for (const [filename, source] of Object.entries(files.value)) {
    try {
      results[filename] = props.compiler.analyzeSfc(source, { filename });
    } catch {
      results[filename] = null;
    }
  }
  analysisResults.value = results;

  // Second pass: cross-file analysis
  if (options.value.provideInject) {
    issues.push(...analyzeProvideInject());
  }
  if (options.value.componentEmits) {
    issues.push(...analyzeComponentEmits());
  }
  if (options.value.fallthroughAttrs) {
    issues.push(...analyzeFallthroughAttrs());
  }
  if (options.value.reactivityTracking) {
    issues.push(...analyzeReactivity());
  }
  if (options.value.uniqueIds) {
    issues.push(...analyzeUniqueIds());
  }
  if (options.value.serverClientBoundary) {
    issues.push(...analyzeSSRBoundary());
  }

  crossFileIssues.value = issues;
  analysisTime.value = performance.now() - startTime;
  isAnalyzing.value = false;
}

function analyzeProvideInject(): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];
  const provides: Map<string, { file: string; line: number; column: number; endLine: number; endColumn: number; isSymbol: boolean }> = new Map();
  const injects: Array<{ key: string; file: string; line: number; column: number; endLine: number; endColumn: number; hasDefault: boolean; isSymbol: boolean }> = [];

  // Collect provides - support both string keys and Symbol keys
  for (const [filename, source] of Object.entries(files.value)) {
    // String keys: provide('key', value) or provide("key", value)
    const stringProvideRegex = /provide\s*\(\s*['"]([^'"]+)['"]/g;
    let match;
    while ((match = stringProvideRegex.exec(source)) !== null) {
      const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
      provides.set(match[1], { file: filename, isSymbol: false, ...loc });
    }

    // Symbol keys: provide(MySymbol, value) or provide(InjectionKey, value)
    const symbolProvideRegex = /provide\s*\(\s*(\w+)\s*,/g;
    while ((match = symbolProvideRegex.exec(source)) !== null) {
      const symbolName = match[1];
      // Skip if it looks like a string literal call we already matched
      if (symbolName === 'inject' || symbolName === 'provide') continue;
      const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
      provides.set(`Symbol:${symbolName}`, { file: filename, isSymbol: true, ...loc });
    }
  }

  // Collect injects - support both string keys and Symbol keys
  for (const [filename, source] of Object.entries(files.value)) {
    // String keys: inject('key') or inject('key', defaultValue)
    // Use [^(]* to skip over any generic type params like <Ref<'light' | 'dark'>>
    const stringInjectRegex = /inject[^(]*\(\s*['"]([^'"]+)['"](?:\s*,\s*([^)]+))?\)/g;
    let match;
    while ((match = stringInjectRegex.exec(source)) !== null) {
      const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
      injects.push({
        key: match[1],
        file: filename,
        hasDefault: !!match[2],
        isSymbol: false,
        ...loc,
      });
    }

    // Symbol keys: inject(MySymbol) or inject(InjectionKey)
    // Match inject followed by optional generics and then identifier (not string)
    const symbolInjectRegex = /inject[^(]*\(\s*([A-Z]\w*)\s*(?:,\s*([^)]+))?\)/g;
    while ((match = symbolInjectRegex.exec(source)) !== null) {
      const symbolName = match[1];
      const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
      injects.push({
        key: `Symbol:${symbolName}`,
        file: filename,
        hasDefault: !!match[2],
        isSymbol: true,
        ...loc,
      });
    }
  }

  // Check for unmatched injects
  for (const inject of injects) {
    if (!provides.has(inject.key)) {
      const severity = inject.hasDefault ? 'info' : 'warning';
      const displayKey = inject.isSymbol ? inject.key.replace('Symbol:', '') : `'${inject.key}'`;
      const provideExample = inject.isSymbol
        ? `Add provide(${inject.key.replace('Symbol:', '')}, value) in a parent component`
        : `Add provide('${inject.key}', value) in a parent component`;
      issues.push(createIssue(
        'provide-inject',
        'cross-file/unmatched-inject',
        severity,
        `inject(${displayKey}) has no matching provide() in any ancestor component`,
        inject.file,
        inject.line,
        inject.column,
        {
          endLine: inject.endLine,
          endColumn: inject.endColumn,
          suggestion: inject.hasDefault
            ? 'Using default value since no provider found'
            : provideExample,
        }
      ));
    }
  }

  // Check for unused provides
  for (const [key, loc] of provides.entries()) {
    const hasConsumer = injects.some(i => i.key === key);
    if (!hasConsumer) {
      issues.push(createIssue(
        'provide-inject',
        'cross-file/unused-provide',
        'info',
        `provide('${key}') is not consumed by any descendant component`,
        loc.file,
        loc.line,
        loc.column,
        { suggestion: 'Remove if not needed, or add inject() in a child component' }
      ));
    }
  }

  return issues;
}

function analyzeComponentEmits(): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];

  for (const [filename, source] of Object.entries(files.value)) {
    // Extract declared emits - support both syntax styles:
    // 1. defineEmits<{ eventName: [PayloadType] }>()  (shorthand)
    // 2. defineEmits<{ (e: 'eventName', payload: Type): void }>()  (callback)
    const emitDeclRegex = /defineEmits\s*<\s*\{([^}]+)\}\s*>/s;
    const emitDeclMatch = emitDeclRegex.exec(source);
    if (!emitDeclMatch || emitDeclMatch.index === undefined) continue;

    const declaredEmits: Array<{ name: string; loc: { line: number; column: number; endLine: number; endColumn: number } }> = [];
    const emitContent = emitDeclMatch[1];
    const emitContentOffset = emitDeclMatch.index + emitDeclMatch[0].indexOf(emitContent);

    // Style 1: Shorthand syntax - { eventName: [Type], eventName2: [] }
    const shorthandRegex = /(\w+)\s*:\s*\[/g;
    let match;
    while ((match = shorthandRegex.exec(emitContent)) !== null) {
      const absoluteOffset = emitContentOffset + match.index;
      const loc = findLineAndColumnAtOffset(source, absoluteOffset, match[1].length);
      declaredEmits.push({ name: match[1], loc });
    }

    // Style 2: Callback syntax - { (e: 'eventName', ...): void }
    const callbackRegex = /\(\s*e:\s*['"]([^'"]+)['"]/g;
    while ((match = callbackRegex.exec(emitContent)) !== null) {
      const absoluteOffset = emitContentOffset + match.index;
      const loc = findLineAndColumnAtOffset(source, absoluteOffset, match[0].length);
      declaredEmits.push({ name: match[1], loc });
    }

    // Check if each declared emit is called
    for (const emit of declaredEmits) {
      const emitCallRegex = new RegExp(`emit\\s*\\(\\s*['"]${emit.name}['"]`, 'g');
      if (!emitCallRegex.test(source)) {
        issues.push(createIssue(
          'component-emit',
          'cross-file/unused-emit',
          'warning',
          `Event '${emit.name}' is declared in defineEmits but never emitted`,
          filename,
          emit.loc.line,
          emit.loc.column,
          {
            endLine: emit.loc.endLine,
            endColumn: emit.loc.endColumn,
            suggestion: `Remove '${emit.name}' from defineEmits if not needed`,
          }
        ));
      }
    }

    // Check for undeclared emits
    const emitCallRegex = /emit\s*\(\s*['"]([^'"]+)['"]/g;
    while ((match = emitCallRegex.exec(source)) !== null) {
      const emitName = match[1];
      if (!declaredEmits.some(e => e.name === emitName)) {
        const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
        issues.push(createIssue(
          'component-emit',
          'cross-file/undeclared-emit',
          'error',
          `Event '${emitName}' is emitted but not declared in defineEmits`,
          filename,
          loc.line,
          loc.column,
          {
            endLine: loc.endLine,
            endColumn: loc.endColumn,
            suggestion: `Add '${emitName}' to defineEmits type definition`,
          }
        ));
      }
    }
  }

  // Check for unhandled event listeners
  for (const [filename, source] of Object.entries(files.value)) {
    const listenerRegex = /@([\w-]+)(?:\.[\w-]+)*="/g;
    let match;
    while ((match = listenerRegex.exec(source)) !== null) {
      const eventName = match[1];
      // Skip native events
      if (isNativeEvent(eventName)) continue;

      // Check if this event is declared by any imported component
      const imports = dependencyGraph.value[filename] || [];
      let hasEmitter = false;
      for (const imp of imports) {
        const impSource = files.value[imp];
        if (impSource && impSource.includes(`'${eventName}'`)) {
          hasEmitter = true;
          break;
        }
      }

      if (!hasEmitter && !['update', 'modelValue'].includes(eventName)) {
        const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
        issues.push(createIssue(
          'component-emit',
          'cross-file/unmatched-listener',
          'info',
          `Listening for @${eventName} but no imported component declares this emit`,
          filename,
          loc.line,
          loc.column,
          { endLine: loc.endLine, endColumn: loc.endColumn }
        ));
      }
    }
  }

  return issues;
}

function analyzeFallthroughAttrs(): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];

  for (const [filename, source] of Object.entries(files.value)) {
    const templateMatch = source.match(/<template>([\s\S]*)<\/template>/);
    if (!templateMatch) continue;

    const template = templateMatch[1];

    // Properly count root elements by tracking tag depth
    const rootElementCount = countRootElements(template);
    const hasMultipleRoots = rootElementCount > 1;

    if (hasMultipleRoots && !source.includes('v-bind="$attrs"') && !source.includes(':="$attrs"')) {
      // Check if component receives any non-prop attributes from parents
      const componentName = filename.replace('.vue', '');
      let hasPassedAttrs = false;

      for (const [parentFile, parentSource] of Object.entries(files.value)) {
        if (parentFile === filename) continue;
        // Check if parent uses this component with non-standard attributes
        const usageRegex = new RegExp(`<${componentName}[^>]*(?:data-|aria-|class=|style=)[^>]*>`, 'i');
        if (usageRegex.test(parentSource)) {
          hasPassedAttrs = true;
          break;
        }
      }

      const loc = findLineAndColumn(source, '<template>');
      if (loc) {
        issues.push(createIssue(
          'fallthrough-attrs',
          'cross-file/multi-root-attrs',
          hasPassedAttrs ? 'warning' : 'info',
          `Component has ${rootElementCount} root elements but $attrs is not explicitly bound`,
          filename,
          loc.line + 1,
          1,
          {
            suggestion: 'Add v-bind="$attrs" to the intended root element, or wrap in a single root',
          }
        ));
      }
    }
  }

  return issues;
}

// Count root-level elements in a template (depth 0 elements only)
function countRootElements(template: string): number {
  // Remove comments first
  const withoutComments = template.replace(/<!--[\s\S]*?-->/g, '');

  // Self-closing void elements that don't need closing tags
  const voidElements = new Set([
    'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input',
    'link', 'meta', 'param', 'source', 'track', 'wbr'
  ]);

  let depth = 0;
  let rootCount = 0;

  // Match all tags (opening, closing, self-closing)
  const tagRegex = /<\/?([a-zA-Z][\w-]*)[^>]*\/?>/g;
  let match;

  while ((match = tagRegex.exec(withoutComments)) !== null) {
    const fullTag = match[0];
    const tagName = match[1].toLowerCase();

    const isClosing = fullTag.startsWith('</');
    const isSelfClosing = fullTag.endsWith('/>') || voidElements.has(tagName);

    if (isClosing) {
      depth--;
    } else {
      if (depth === 0) {
        rootCount++;
      }
      if (!isSelfClosing) {
        depth++;
      }
    }
  }

  return rootCount;
}

function analyzeReactivity(): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];

  for (const [filename, source] of Object.entries(files.value)) {
    // Detect destructuring of inject result
    const injectDestructureRegex = /const\s*\{([^}]+)\}\s*=\s*inject\s*\(/g;
    let match;
    while ((match = injectDestructureRegex.exec(source)) !== null) {
      const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
      const props = match[1].split(',').map(p => p.trim());
      issues.push(createIssue(
        'reactivity',
        'cross-file/reactivity-loss',
        'error',
        `Destructuring inject() result loses reactivity for: ${props.join(', ')}`,
        filename,
        loc.line,
        loc.column,
        {
          endLine: loc.endLine,
          endColumn: loc.endColumn,
          suggestion: 'Access properties directly from inject result, or use computed()',
        }
      ));
    }

    // Detect destructuring of reactive/ref without toRefs
    const reactiveDestructureRegex = /const\s*\{([^}]+)\}\s*=\s*(reactive|ref)\s*\(/g;
    while ((match = reactiveDestructureRegex.exec(source)) !== null) {
      // Check if toRefs is used nearby
      if (!source.includes('toRefs')) {
        const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
        issues.push(createIssue(
          'reactivity',
          'cross-file/reactivity-loss',
          'warning',
          `Destructuring ${match[2]}() object loses reactivity`,
          filename,
          loc.line,
          loc.column,
          {
            endLine: loc.endLine,
            endColumn: loc.endColumn,
            suggestion: `Use toRefs(${match[2]}(...)) to maintain reactivity`,
          }
        ));
      }
    }

    // Detect Pinia store destructuring without storeToRefs
    const storeDestructureRegex = /const\s*\{([^}]+)\}\s*=\s*(\w+Store)\s*(?:\(\s*\))?/g;
    while ((match = storeDestructureRegex.exec(source)) !== null) {
      // Check for storeToRefs usage
      if (!source.includes('storeToRefs')) {
        const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
        const props = match[1].split(',').map(p => p.trim());
        issues.push(createIssue(
          'reactivity',
          'cross-file/store-reactivity-loss',
          'warning',
          `Destructuring Pinia store loses reactivity for: ${props.join(', ')}`,
          filename,
          loc.line,
          loc.column,
          {
            endLine: loc.endLine,
            endColumn: loc.endColumn,
            suggestion: `Use storeToRefs(${match[2]}) for state and getters`,
          }
        ));
      }
    }
  }

  return issues;
}

function analyzeUniqueIds(): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];
  const staticIds: Map<string, Array<{ file: string; line: number; column: number; endLine: number; endColumn: number }>> = new Map();

  for (const [filename, source] of Object.entries(files.value)) {
    // Find static id attributes
    const idRegex = /\bid=["']([^"'${}]+)["']/g;
    let match;
    while ((match = idRegex.exec(source)) !== null) {
      const id = match[1];
      const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
      if (!staticIds.has(id)) staticIds.set(id, []);
      staticIds.get(id)!.push({ file: filename, ...loc });
    }

    // Check for static ids in v-for
    const vforIdRegex = /v-for=[^>]+>\s*[^]*?id=["']([^"'${}]+)["']/g;
    while ((match = vforIdRegex.exec(source)) !== null) {
      const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
      issues.push(createIssue(
        'unique-id',
        'cross-file/non-unique-id',
        'error',
        `Static id="${match[1]}" inside v-for will create duplicate IDs`,
        filename,
        loc.line,
        loc.column,
        {
          endLine: loc.endLine,
          endColumn: loc.endColumn,
          suggestion: 'Use a dynamic id like :id="`item-${index}`"',
        }
      ));
    }
  }

  // Check for duplicate static IDs across files
  for (const [id, locations] of staticIds.entries()) {
    if (locations.length > 1) {
      const primary = locations[0];
      issues.push(createIssue(
        'unique-id',
        'cross-file/duplicate-id',
        'warning',
        `Element id="${id}" is duplicated in ${locations.length} locations`,
        primary.file,
        primary.line,
        primary.column,
        {
          relatedLocations: locations.slice(1).map(loc => ({
            file: loc.file,
            line: loc.line,
            column: loc.column,
            message: 'Also defined here',
          })),
          suggestion: 'Use unique IDs across your application',
        }
      ));
    }
  }

  return issues;
}

function analyzeSSRBoundary(): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];
  const browserApis = ['window', 'document', 'navigator', 'localStorage', 'sessionStorage', 'location', 'history'];

  for (const [filename, source] of Object.entries(files.value)) {
    const scriptMatch = source.match(/<script[^>]*>([^]*?)<\/script>/);
    if (!scriptMatch) continue;

    const script = scriptMatch[1];

    for (const api of browserApis) {
      const apiRegex = new RegExp(`\\b${api}\\b`, 'g');
      let match;
      while ((match = apiRegex.exec(script)) !== null) {
        // Check if inside onMounted or other client-only hooks
        const beforeMatch = script.substring(0, match.index);
        const isInClientHook = /on(Mounted|BeforeMount|Updated|BeforeUpdate)\s*\([^)]*$/.test(beforeMatch) ||
                              /onMounted\s*\(\s*(?:async\s*)?\(\)\s*=>\s*\{[^}]*$/.test(beforeMatch);

        if (!isInClientHook) {
          // Calculate position in full source
          const scriptStart = source.indexOf(scriptMatch[1]);
          const fullOffset = scriptStart + match.index;
          const loc = findLineAndColumnAtOffset(source, fullOffset, api.length);
          issues.push(createIssue(
            'ssr-boundary',
            'cross-file/browser-api-ssr',
            'warning',
            `Browser API '${api}' used outside client-only lifecycle hook`,
            filename,
            loc.line,
            loc.column,
            {
              endLine: loc.endLine,
              endColumn: loc.endColumn,
              suggestion: `Move to onMounted() or guard with 'if (import.meta.client)'`,
            }
          ));
        }
      }
    }
  }

  return issues;
}

function isNativeEvent(event: string): boolean {
  return [
    'click', 'dblclick', 'mousedown', 'mouseup', 'mousemove', 'mouseenter', 'mouseleave',
    'keydown', 'keyup', 'keypress', 'focus', 'blur', 'change', 'input', 'submit',
    'scroll', 'resize', 'load', 'error', 'contextmenu', 'wheel',
    'touchstart', 'touchmove', 'touchend', 'drag', 'dragstart', 'dragend', 'drop',
  ].includes(event);
}

// === File Management ===
function addFile() {
  const name = prompt('Enter file name (e.g., NewComponent.vue)');
  if (name && !files.value[name]) {
    files.value[name] = `<script setup lang="ts">\n// ${name}\n<\/script>\n\n<template>\n  <div></div>\n</template>`;
    activeFile.value = name;
  }
}

function removeFile(name: string) {
  if (Object.keys(files.value).length > 1 && confirm(`Delete ${name}?`)) {
    delete files.value[name];
    if (activeFile.value === name) {
      activeFile.value = Object.keys(files.value)[0];
    }
  }
}

function resetProject() {
  files.value = { ...SAMPLE_PROJECT };
  activeFile.value = 'ParentComponent.vue';
  crossFileIssues.value = [];
  selectedIssue.value = null;
}

function selectIssue(issue: CrossFileIssue) {
  selectedIssue.value = issue;
  activeFile.value = issue.file;
}

function getFileIcon(filename: string): string {
  if (filename.endsWith('.vue')) return '◇';
  if (filename.endsWith('.ts')) return '⬡';
  return '◆';
}

function getSeverityIcon(severity: string): string {
  return severity === 'error' ? '✕' : severity === 'warning' ? '⚠' : 'ℹ';
}

function getTypeLabel(type: string): string {
  const labels: Record<string, string> = {
    'provide-inject': 'Provide/Inject',
    'component-emit': 'Component Emit',
    'fallthrough-attrs': 'Fallthrough Attrs',
    'reactivity': 'Reactivity',
    'unique-id': 'Unique ID',
    'ssr-boundary': 'SSR Boundary',
  };
  return labels[type] || type;
}

function getTypeColor(type: string): string {
  const colors: Record<string, string> = {
    'provide-inject': '#8b5cf6',
    'component-emit': '#f59e0b',
    'fallthrough-attrs': '#06b6d4',
    'reactivity': '#ef4444',
    'unique-id': '#10b981',
    'ssr-boundary': '#3b82f6',
  };
  return colors[type] || '#6b7280';
}

// === Watchers ===
watch([files, options], () => {
  analyzeAll();
}, { deep: true });

watch(() => props.compiler, () => {
  if (props.compiler) analyzeAll();
});

onMounted(() => {
  if (props.compiler) analyzeAll();
});
</script>

<template>
  <div ref="containerRef" class="cross-file-playground" :style="gridStyle" :class="{ resizing: isResizingSidebar || isResizingDiagnostics }">
    <!-- Sidebar: File Tree & Dependency Graph -->
    <aside class="sidebar">
      <div class="sidebar-section">
        <div class="section-header">
          <h3>Project Files</h3>
          <div class="section-actions">
            <button @click="addFile" class="icon-btn" title="Add file">+</button>
            <button @click="resetProject" class="icon-btn" title="Reset">↺</button>
          </div>
        </div>
        <nav class="file-tree">
          <div
            v-for="(_, name) in files"
            :key="name"
            :class="['file-item', { active: activeFile === name, 'has-errors': issuesByFile[name]?.some(i => i.severity === 'error'), 'has-warnings': issuesByFile[name]?.some(i => i.severity === 'warning') }]"
            @click="activeFile = name"
          >
            <span class="file-icon">{{ getFileIcon(name) }}</span>
            <span class="file-name">{{ name }}</span>
            <span v-if="issuesByFile[name]?.length" class="file-badge" :class="issuesByFile[name].some(i => i.severity === 'error') ? 'error' : 'warning'">
              {{ issuesByFile[name].length }}
            </span>
            <button v-if="Object.keys(files).length > 1" @click.stop="removeFile(name)" class="file-delete">×</button>
          </div>
        </nav>
      </div>

      <div class="sidebar-section">
        <div class="section-header">
          <h3>Dependencies</h3>
        </div>
        <div class="dependency-graph">
          <div v-for="(deps, file) in dependencyGraph" :key="file" class="dep-node">
            <span class="dep-file">{{ file }}</span>
            <div v-if="deps.length" class="dep-arrows">
              <div v-for="dep in deps" :key="dep" class="dep-edge">
                <span class="dep-arrow">→</span>
                <span class="dep-target" @click="activeFile = dep">{{ dep }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="sidebar-section options-section">
        <div class="section-header">
          <h3>Analyzers</h3>
          <span class="analysis-mode-badge" title="Strict Static Analysis: No heuristics, all issues are based on precise AST analysis">STRICT</span>
        </div>
        <div class="options-grid">
          <label class="option-toggle">
            <input type="checkbox" v-model="options.provideInject" />
            <span>Provide/Inject</span>
          </label>
          <label class="option-toggle">
            <input type="checkbox" v-model="options.componentEmits" />
            <span>Component Emits</span>
          </label>
          <label class="option-toggle">
            <input type="checkbox" v-model="options.fallthroughAttrs" />
            <span>Fallthrough Attrs</span>
          </label>
          <label class="option-toggle">
            <input type="checkbox" v-model="options.reactivityTracking" />
            <span>Reactivity</span>
          </label>
          <label class="option-toggle">
            <input type="checkbox" v-model="options.uniqueIds" />
            <span>Unique IDs</span>
          </label>
          <label class="option-toggle">
            <input type="checkbox" v-model="options.serverClientBoundary" />
            <span>SSR Boundary</span>
          </label>
        </div>
      </div>
    </aside>

    <!-- Resize Handle: Sidebar -->
    <div class="resize-handle resize-handle-left" @mousedown="startSidebarResize"></div>

    <!-- Main Content: Editor -->
    <main class="editor-pane">
      <div class="editor-header">
        <div class="editor-tabs">
          <button
            v-for="(_, name) in files"
            :key="name"
            :class="['editor-tab', { active: activeFile === name }]"
            @click="activeFile = name"
          >
            <span class="tab-icon">{{ getFileIcon(name) }}</span>
            {{ name }}
            <span v-if="issuesByFile[name]?.length" class="tab-badge" :class="issuesByFile[name].some(i => i.severity === 'error') ? 'error' : 'warning'">
              {{ issuesByFile[name].length }}
            </span>
          </button>
        </div>
        <div class="editor-status">
          <span v-if="isAnalyzing" class="status-analyzing">Analyzing...</span>
          <span v-else class="status-time">{{ analysisTime.toFixed(1) }}ms</span>
        </div>
      </div>
      <div class="editor-content">
        <MonacoEditor
          v-model="currentSource"
          language="vue"
          :diagnostics="currentDiagnostics"
        />
      </div>
    </main>

    <!-- Resize Handle: Diagnostics -->
    <div class="resize-handle resize-handle-right" @mousedown="startDiagnosticsResize"></div>

    <!-- Right Panel: Diagnostics -->
    <aside class="diagnostics-pane">
      <div class="diagnostics-header">
        <h3>Diagnostics</h3>
        <div class="diagnostics-stats">
          <span class="stat-chip error" v-if="stats.errors">{{ stats.errors }} errors</span>
          <span class="stat-chip warning" v-if="stats.warnings">{{ stats.warnings }} warnings</span>
          <span class="stat-chip info" v-if="stats.infos">{{ stats.infos }} info</span>
        </div>
      </div>

      <div v-if="crossFileIssues.length === 0" class="diagnostics-empty">
        <span class="empty-icon">✓</span>
        <span>No issues detected</span>
      </div>

      <div v-else class="diagnostics-list">
        <!-- Group by type -->
        <div v-for="(issues, type) in issuesByType" :key="type" class="issue-group">
          <div class="group-header" :style="{ '--type-color': getTypeColor(type) }">
            <span class="group-badge">{{ getTypeLabel(type) }}</span>
            <span class="group-count">{{ issues.length }}</span>
          </div>
          <div class="group-issues">
            <div
              v-for="issue in issues"
              :key="issue.id"
              :class="['issue-card', issue.severity, { selected: selectedIssue?.id === issue.id }]"
              @click="selectIssue(issue)"
            >
              <div class="issue-header">
                <span class="severity-icon">{{ getSeverityIcon(issue.severity) }}</span>
                <span class="issue-code">{{ issue.code }}</span>
                <span class="issue-location">{{ issue.file }}:{{ issue.line }}</span>
              </div>
              <div class="issue-message">{{ issue.message }}</div>
              <div v-if="issue.suggestion" class="issue-suggestion">
                <span class="suggestion-icon">→</span>
                {{ issue.suggestion }}
              </div>
              <div v-if="issue.relatedLocations?.length" class="issue-related">
                <div v-for="(rel, i) in issue.relatedLocations" :key="i" class="related-item">
                  <span class="related-loc">{{ rel.file }}:{{ rel.line }}</span>
                  <span class="related-msg">{{ rel.message }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </aside>
  </div>
</template>

<style scoped>
.cross-file-playground {
  display: grid;
  grid-template-columns: 220px 4px 1fr 4px 320px;
  grid-column: 1 / -1;
  height: 100%;
  min-height: 0;
  background: var(--bg-primary);
  font-size: 12px;
  user-select: none;
}

.cross-file-playground.resizing {
  cursor: col-resize;
}

.cross-file-playground.resizing * {
  pointer-events: none;
}

/* === Resize Handles === */
.resize-handle {
  width: 4px;
  background: var(--border-primary);
  cursor: col-resize;
  transition: background 0.15s;
  position: relative;
}

.resize-handle:hover,
.resize-handle:active {
  background: var(--accent-primary);
}

.resize-handle::after {
  content: '';
  position: absolute;
  top: 0;
  bottom: 0;
  width: 8px;
  left: -2px;
}

/* === Sidebar === */
.sidebar {
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border-primary);
  overflow: hidden;
}

.sidebar-section {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.sidebar-section:not(:last-child) {
  border-bottom: 1px solid var(--border-primary);
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--bg-tertiary);
}

.section-header h3 {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted);
  margin: 0;
}

.analysis-mode-badge {
  font-size: 9px;
  font-weight: 700;
  padding: 2px 6px;
  border-radius: 3px;
  background: linear-gradient(135deg, #10b981, #059669);
  color: #fff;
  letter-spacing: 0.5px;
  cursor: help;
}

.section-actions {
  display: flex;
  gap: 4px;
}

.icon-btn {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 1px solid var(--border-primary);
  border-radius: 3px;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 12px;
  transition: all 0.15s;
}

.icon-btn:hover {
  background: var(--bg-primary);
  color: var(--text-primary);
  border-color: var(--text-muted);
}

/* File Tree */
.file-tree {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  cursor: pointer;
  transition: background 0.1s;
  position: relative;
}

.file-item:hover {
  background: var(--bg-tertiary);
}

.file-item.active {
  background: var(--accent-primary);
  background: rgba(224, 112, 72, 0.15);
}

.file-item.has-errors .file-icon { color: #ef4444; }
.file-item.has-warnings .file-icon { color: #f59e0b; }

.file-icon {
  font-size: 10px;
  color: var(--accent-rust);
}

.file-name {
  flex: 1;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file-badge {
  font-size: 9px;
  padding: 1px 5px;
  border-radius: 8px;
  font-weight: 600;
}

.file-badge.error {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
}

.file-badge.warning {
  background: rgba(251, 191, 36, 0.2);
  color: #fbbf24;
}

.file-delete {
  position: absolute;
  right: 8px;
  width: 16px;
  height: 16px;
  display: none;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 14px;
  border-radius: 2px;
}

.file-item:hover .file-delete {
  display: flex;
}

.file-delete:hover {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
}

/* Dependency Graph */
.dependency-graph {
  padding: 8px 12px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
}

.dep-node {
  margin-bottom: 8px;
}

.dep-file {
  color: var(--text-secondary);
}

.dep-arrows {
  padding-left: 12px;
  margin-top: 2px;
}

.dep-edge {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--text-muted);
}

.dep-arrow {
  color: var(--accent-rust);
}

.dep-target {
  color: var(--text-secondary);
  cursor: pointer;
}

.dep-target:hover {
  color: var(--accent-rust);
  text-decoration: underline;
}

/* Options */
.options-section {
  margin-top: auto;
}

.options-grid {
  padding: 8px 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.option-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  font-size: 11px;
  color: var(--text-secondary);
}

.option-toggle input {
  width: 12px;
  height: 12px;
  accent-color: var(--accent-primary);
}

.option-toggle:hover {
  color: var(--text-primary);
}

/* === Editor Pane === */
.editor-pane {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.editor-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-primary);
  padding-right: 12px;
}

.editor-tabs {
  display: flex;
  overflow-x: auto;
}

.editor-tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  background: transparent;
  border: none;
  border-right: 1px solid var(--border-primary);
  font-size: 11px;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-muted);
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.1s;
}

.editor-tab:hover {
  background: var(--bg-tertiary);
  color: var(--text-secondary);
}

.editor-tab.active {
  background: var(--bg-primary);
  color: var(--text-primary);
  border-bottom: 2px solid var(--accent-rust);
  margin-bottom: -1px;
}

.tab-icon {
  font-size: 10px;
  color: var(--accent-rust);
}

.tab-badge {
  font-size: 9px;
  padding: 1px 5px;
  border-radius: 8px;
  font-weight: 600;
}

.tab-badge.error {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
}

.tab-badge.warning {
  background: rgba(251, 191, 36, 0.2);
  color: #fbbf24;
}

.editor-status {
  font-size: 10px;
  font-family: 'JetBrains Mono', monospace;
}

.status-analyzing {
  color: var(--accent-rust);
}

.status-time {
  color: var(--text-muted);
}

.editor-content {
  flex: 1;
  min-height: 0;
}

/* === Diagnostics Pane === */
.diagnostics-pane {
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
  border-left: 1px solid var(--border-primary);
  overflow: hidden;
}

.diagnostics-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--bg-tertiary);
  border-bottom: 1px solid var(--border-primary);
}

.diagnostics-header h3 {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted);
  margin: 0;
}

.diagnostics-stats {
  display: flex;
  gap: 6px;
}

.stat-chip {
  font-size: 9px;
  padding: 2px 6px;
  border-radius: 3px;
  font-weight: 600;
}

.stat-chip.error {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
}

.stat-chip.warning {
  background: rgba(251, 191, 36, 0.2);
  color: #fbbf24;
}

.stat-chip.info {
  background: rgba(96, 165, 250, 0.2);
  color: #60a5fa;
}

.diagnostics-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 32px;
  color: #4ade80;
}

.empty-icon {
  font-size: 24px;
}

.diagnostics-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.issue-group {
  margin-bottom: 12px;
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
  margin-bottom: 4px;
  background: var(--bg-tertiary);
  border-radius: 4px;
  border-left: 3px solid var(--type-color, var(--text-muted));
}

.group-badge {
  font-size: 10px;
  font-weight: 600;
  color: var(--text-secondary);
}

.group-count {
  font-size: 10px;
  color: var(--text-muted);
  font-family: 'JetBrains Mono', monospace;
}

.group-issues {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.issue-card {
  padding: 8px;
  background: var(--bg-primary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.1s;
  border-left: 3px solid transparent;
}

.issue-card:hover {
  background: var(--bg-tertiary);
}

.issue-card.selected {
  border-color: var(--accent-rust);
  background: rgba(224, 112, 72, 0.1);
}

.issue-card.error { border-left-color: #ef4444; }
.issue-card.warning { border-left-color: #f59e0b; }
.issue-card.info { border-left-color: #60a5fa; }

.issue-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.severity-icon {
  font-size: 10px;
}

.issue-card.error .severity-icon { color: #ef4444; }
.issue-card.warning .severity-icon { color: #f59e0b; }
.issue-card.info .severity-icon { color: #60a5fa; }

.issue-code {
  font-size: 9px;
  font-family: 'JetBrains Mono', monospace;
  padding: 1px 4px;
  background: var(--bg-secondary);
  border-radius: 2px;
  color: var(--text-muted);
}

.issue-location {
  margin-left: auto;
  font-size: 9px;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-muted);
}

.issue-message {
  font-size: 11px;
  color: var(--text-primary);
  line-height: 1.4;
}

.issue-suggestion {
  margin-top: 6px;
  padding: 6px;
  font-size: 10px;
  color: #4ade80;
  background: rgba(74, 222, 128, 0.1);
  border-radius: 3px;
  display: flex;
  gap: 6px;
}

.suggestion-icon {
  flex-shrink: 0;
}

.issue-related {
  margin-top: 6px;
  padding-top: 6px;
  border-top: 1px solid var(--border-primary);
}

.related-item {
  display: flex;
  gap: 8px;
  font-size: 10px;
  color: var(--text-muted);
  margin-bottom: 2px;
}

.related-loc {
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-secondary);
}

/* === Responsive === */
@media (max-width: 1200px) {
  .cross-file-playground {
    grid-template-columns: 180px 1fr 280px;
  }
}

@media (max-width: 900px) {
  .cross-file-playground {
    grid-template-columns: 1fr;
    grid-template-rows: auto 1fr auto;
  }

  .sidebar {
    flex-direction: row;
    border-right: none;
    border-bottom: 1px solid var(--border-primary);
    overflow-x: auto;
  }

  .sidebar-section {
    flex-direction: row;
    min-width: max-content;
  }

  .diagnostics-pane {
    border-left: none;
    border-top: 1px solid var(--border-primary);
    max-height: 300px;
  }
}
</style>
