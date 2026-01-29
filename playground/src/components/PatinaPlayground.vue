<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted } from 'vue';
import MonacoEditor from './MonacoEditor.vue';
import * as monaco from 'monaco-editor';
import type { WasmModule, LintResult, LintDiagnostic, LintRule, LocaleInfo } from '../wasm/index';
import { getWasm } from '../wasm/index';

interface Diagnostic {
  message: string;
  help?: string;
  startLine: number;
  startColumn: number;
  endLine?: number;
  endColumn?: number;
  severity: 'error' | 'warning' | 'info';
}

const props = defineProps<{
  compiler: WasmModule | null;
}>();

const LINT_PRESET = `<script setup lang="ts">
import { ref } from 'vue'

const items = ref([
  { name: 'Item 1' },
  { name: 'Item 2' },
])

const users = ref([
  { id: 1, name: 'Alice', active: true },
  { id: 2, name: 'Bob', active: false },
])

const products = ref([
  { id: 1, name: 'Product A', inStock: true },
  { id: 2, name: 'Product B', inStock: false },
])

const htmlContent = '<b>Hello</b>'
const handleClick = () => {}
<\/script>

<template>
  <div class="container">
    <!-- vue/require-v-for-key: Missing :key attribute -->
    <ul>
      <li v-for="item in items">{{ item.name }}</li>
    </ul>

    <!-- vue/no-use-v-if-with-v-for: v-if with v-for on same element -->
    <div v-for="user in users" v-if="user.active" :key="user.id">
      {{ user.name }}
    </div>

    <!-- a11y/img-alt: Missing alt attribute -->
    <img src="/logo.png" />

    <!-- a11y/anchor-has-content: Empty anchor -->
    <a href="/home"></a>

    <!-- a11y/heading-has-content: Empty heading -->
    <h1></h1>

    <!-- a11y/click-events-have-key-events: Click without keyboard handler -->
    <div @click="handleClick">Click me</div>

    <!-- a11y/tabindex-no-positive: Positive tabindex -->
    <button tabindex="5">Bad Tab Order</button>

    <!-- a11y/form-control-has-label: Input without label -->
    <input type="text" placeholder="Enter name" />

    <!-- a11y/aria-props: Invalid ARIA attribute (typo) -->
    <input aria-labeledby="label-id" />

    <!-- a11y/aria-role: Invalid ARIA role -->
    <div role="datepicker"></div>

    <!-- a11y/aria-role: Abstract ARIA role -->
    <div role="range"></div>

    <!-- vue/use-unique-element-ids: Static id attribute -->
    <label for="user-input">Username:</label>
    <input id="user-input" type="text" />

    <!-- vue/no-v-html: XSS risk -->
    <div v-html="htmlContent"></div>

    <!-- Valid code for comparison -->
    <template v-for="product in products" :key="product.id">
      <div v-if="product.inStock">
        {{ product.name }}
      </div>
    </template>
  </div>
</template>

<style scoped>
.container {
  padding: 20px;
}
</style>
`;

const source = ref(LINT_PRESET);
const lintResult = ref<LintResult | null>(null);
const rules = ref<LintRule[]>([]);
const error = ref<string | null>(null);
const activeTab = ref<'diagnostics' | 'rules'>('diagnostics');
const lintTime = ref<number | null>(null);

// Ref to MonacoEditor for direct method calls (workaround for vite-plugin-vize reactivity issue)
const editorRef = ref<InstanceType<typeof MonacoEditor> | null>(null);

// Rule configuration state
const enabledRules = ref<Set<string>>(new Set());
const severityOverrides = ref<Map<string, 'error' | 'warning' | 'off'>>(new Map());
const STORAGE_KEY = 'vize-patina-rules-config';
const LOCALE_STORAGE_KEY = 'vize-patina-locale';

// Locale state
const locales = ref<LocaleInfo[]>([
  { code: 'en', name: 'English' },
  { code: 'ja', name: '日本語' },
  { code: 'zh', name: '中文' },
]);
const currentLocale = ref<'en' | 'ja' | 'zh'>('en');

// Load saved locale preference from localStorage
function loadLocaleConfig() {
  try {
    const saved = localStorage.getItem(LOCALE_STORAGE_KEY);
    if (saved && ['en', 'ja', 'zh'].includes(saved)) {
      currentLocale.value = saved as 'en' | 'ja' | 'zh';
    }
  } catch (e) {
    console.warn('Failed to load locale config:', e);
  }
}

// Save locale preference to localStorage
function saveLocaleConfig() {
  try {
    localStorage.setItem(LOCALE_STORAGE_KEY, currentLocale.value);
  } catch (e) {
    console.warn('Failed to save locale config:', e);
  }
}

// Change locale
function setLocale(locale: 'en' | 'ja' | 'zh') {
  currentLocale.value = locale;
  saveLocaleConfig();
  lint();
}

// Load saved rule configuration from localStorage
function loadRuleConfig() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      const config = JSON.parse(saved);
      enabledRules.value = new Set(config.enabledRules || []);
      severityOverrides.value = new Map(Object.entries(config.severityOverrides || {}));
    }
  } catch (e) {
    console.warn('Failed to load rule config:', e);
  }
}

// Save rule configuration to localStorage
function saveRuleConfig() {
  try {
    const config = {
      enabledRules: Array.from(enabledRules.value),
      severityOverrides: Object.fromEntries(severityOverrides.value),
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
  } catch (e) {
    console.warn('Failed to save rule config:', e);
  }
}

// Initialize all rules as enabled when rules are loaded
function initializeRuleState() {
  if (enabledRules.value.size === 0 && rules.value.length > 0) {
    // Enable all rules by default
    rules.value.forEach(rule => {
      enabledRules.value.add(rule.name);
    });
    saveRuleConfig();
  }
}

// Toggle rule enabled state
function toggleRule(ruleName: string) {
  if (enabledRules.value.has(ruleName)) {
    enabledRules.value.delete(ruleName);
  } else {
    enabledRules.value.add(ruleName);
  }
  saveRuleConfig();
  lint();
}

// Toggle all rules in a category
function toggleCategory(category: string, enabled: boolean) {
  const categoryRules = rules.value.filter(r => r.category === category);
  categoryRules.forEach(rule => {
    if (enabled) {
      enabledRules.value.add(rule.name);
    } else {
      enabledRules.value.delete(rule.name);
    }
  });
  saveRuleConfig();
  lint();
}

// Enable all rules
function enableAllRules() {
  rules.value.forEach(rule => {
    enabledRules.value.add(rule.name);
  });
  saveRuleConfig();
  lint();
}

// Disable all rules
function disableAllRules() {
  enabledRules.value.clear();
  saveRuleConfig();
  lint();
}

// Check if all rules in a category are enabled
function isCategoryFullyEnabled(category: string): boolean {
  const categoryRules = rules.value.filter(r => r.category === category);
  return categoryRules.every(rule => enabledRules.value.has(rule.name));
}

// Check if some rules in a category are enabled
function isCategoryPartiallyEnabled(category: string): boolean {
  const categoryRules = rules.value.filter(r => r.category === category);
  const enabledCount = categoryRules.filter(rule => enabledRules.value.has(rule.name)).length;
  return enabledCount > 0 && enabledCount < categoryRules.length;
}

const errorCount = computed(() => lintResult.value?.errorCount ?? 0);
const warningCount = computed(() => lintResult.value?.warningCount ?? 0);
const enabledRuleCount = computed(() => enabledRules.value.size);

// Calculate template start line offset for correct diagnostic positioning
// The extracted content includes the newline after <template>, so no +1 needed
const templateLineOffset = computed(() => {
  const lines = source.value.split('\n');
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].trim().startsWith('<template')) {
      return i; // Line number (0-indexed) where <template> is
    }
  }
  return 0;
});

// Convert lint diagnostics to Monaco markers
// Note: WASM already returns 1-indexed line/column numbers for the full SFC file
const diagnostics = computed((): Diagnostic[] => {
  if (!lintResult.value?.diagnostics) return [];
  return lintResult.value.diagnostics.map(d => ({
    // Message is already formatted by WASM with [vize:RULE] prefix via i18n
    message: d.message,
    help: d.help,
    startLine: d.location.start.line,
    startColumn: d.location.start.column,
    endLine: d.location.end?.line ?? d.location.start.line,
    endColumn: d.location.end?.column ?? d.location.start.column + 1,
    severity: d.severity,
  }));
});

// Rule filtering
const selectedCategory = ref<string>('all');
const searchQuery = ref('');

const categories = computed(() => {
  const cats = new Set(rules.value.map(r => r.category));
  return ['all', ...Array.from(cats).sort()];
});

const filteredRules = computed(() => {
  return rules.value.filter(rule => {
    const matchesCategory = selectedCategory.value === 'all' || rule.category === selectedCategory.value;
    const matchesSearch = searchQuery.value === '' ||
      rule.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
      rule.description.toLowerCase().includes(searchQuery.value.toLowerCase());
    return matchesCategory && matchesSearch;
  });
});

async function lint() {
  const compiler = getWasm();
  if (!compiler) return;

  const startTime = performance.now();
  error.value = null;

  try {
    const result = compiler.lintSfc(source.value, {
      filename: 'example.vue',
      enabledRules: Array.from(enabledRules.value),
      severityOverrides: Object.fromEntries(severityOverrides.value),
      locale: currentLocale.value,
    });
    lintResult.value = result;
    lintTime.value = performance.now() - startTime;

    // Directly apply diagnostics to editor (workaround for vite-plugin-vize reactivity issue)
    // Use nextTick to ensure computed is updated
    const diags = result?.diagnostics?.map(d => ({
      message: d.message,
      help: d.help,
      startLine: d.location.start.line,
      startColumn: d.location.start.column,
      endLine: d.location.end?.line ?? d.location.start.line,
      endColumn: d.location.end?.column ?? d.location.start.column + 1,
      severity: d.severity,
    })) ?? [];
    editorRef.value?.applyDiagnostics(diags);
  } catch (e) {
    console.error('[Patina] lintSfc error:', e);
    error.value = e instanceof Error ? e.message : String(e);
    lintResult.value = null;
  }
}

function loadRules() {
  const compiler = getWasm();
  if (!compiler) return;

  try {
    rules.value = compiler.getLintRules();
    initializeRuleState();
  } catch (e) {
    console.error('Failed to load rules:', e);
  }
}

// Simple syntax highlighter for code - uses token-based approach to avoid conflicts
function highlightCode(code: string, lang: string): string {
  // Token placeholders to prevent regex conflicts
  const tokens: string[] = [];
  let tokenId = 0;
  const placeholder = (content: string): string => {
    const id = `__TOKEN_${tokenId++}__`;
    tokens.push(content);
    return id;
  };

  let result = code;

  // Vue/HTML specific
  if (lang === 'vue' || lang === 'html') {
    // HTML comments first
    result = result.replace(/(&lt;!--[\s\S]*?--&gt;)/g, (_, m) =>
      placeholder(`<span class="hl-comment">${m}</span>`)
    );
    // Attribute values in quotes (before tags to avoid conflicts)
    result = result.replace(/="([^"]*)"/g, (_, v) =>
      `="${placeholder(`<span class="hl-string">${v}</span>`)}"`
    );
    // Vue directives
    result = result.replace(/(v-[\w-]+|@[\w.-]+|:[\w.-]+(?==")|#[\w.-]+)/g, (_, m) =>
      placeholder(`<span class="hl-directive">${m}</span>`)
    );
    // Tags (opening and closing)
    result = result.replace(/(&lt;\/?)([\w-]+)/g, (_, prefix, tag) =>
      `${prefix}${placeholder(`<span class="hl-tag">${tag}</span>`)}`
    );
    // Mustache interpolation
    result = result.replace(/(\{\{|\}\})/g, (_, m) =>
      placeholder(`<span class="hl-delimiter">${m}</span>`)
    );
  }

  // TypeScript/JavaScript
  if (lang === 'ts' || lang === 'typescript' || lang === 'js' || lang === 'javascript') {
    // Comments first (to avoid highlighting inside comments)
    result = result.replace(/(\/\/.*)/g, (_, m) =>
      placeholder(`<span class="hl-comment">${m}</span>`)
    );
    // Strings (must be before keywords to avoid highlighting keywords inside strings)
    result = result.replace(/('[^']*'|"[^"]*"|`[^`]*`)/g, (_, m) =>
      placeholder(`<span class="hl-string">${m}</span>`)
    );
    // Vue APIs (before general keywords)
    result = result.replace(/\b(ref|reactive|computed|watch|watchEffect|onMounted|onUnmounted|defineProps|defineEmits|toRefs|inject|provide)\b/g, (_, m) =>
      placeholder(`<span class="hl-vue-api">${m}</span>`)
    );
    // Keywords
    result = result.replace(/\b(const|let|var|function|return|if|else|for|while|import|export|from|async|await|new|typeof|instanceof|class|interface|type|extends)\b/g, (_, m) =>
      placeholder(`<span class="hl-keyword">${m}</span>`)
    );
    // Types
    result = result.replace(/\b(string|number|boolean|null|undefined|void|any|never)\b/g, (_, m) =>
      placeholder(`<span class="hl-type">${m}</span>`)
    );
    // Numbers
    result = result.replace(/\b(\d+)\b/g, (_, m) =>
      placeholder(`<span class="hl-number">${m}</span>`)
    );
  }

  // CSS
  if (lang === 'css') {
    // At-rules
    result = result.replace(/(@[\w-]+)/g, (_, m) =>
      placeholder(`<span class="hl-keyword">${m}</span>`)
    );
    // Properties
    result = result.replace(/([\w-]+)(\s*:)/g, (_, prop, colon) =>
      `${placeholder(`<span class="hl-property">${prop}</span>`)}${colon}`
    );
  }

  // Bash
  if (lang === 'bash' || lang === 'sh') {
    // Comments first
    result = result.replace(/(#.*)/g, (_, m) =>
      placeholder(`<span class="hl-comment">${m}</span>`)
    );
    // Commands
    result = result.replace(/\b(npm|yarn|pnpm|git|cd|mkdir|rm|cp|mv|install)\b/g, (_, m) =>
      placeholder(`<span class="hl-keyword">${m}</span>`)
    );
  }

  // Replace all token placeholders with actual content
  for (let i = 0; i < tokens.length; i++) {
    result = result.replace(`__TOKEN_${i}__`, tokens[i]);
  }

  return result;
}

// Simple markdown formatter for help text
function formatHelp(help: string): string {
  let result = help
    // Escape HTML first
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');

  // Code blocks (```lang ... ```)
  result = result.replace(/```(\w*)\n([\s\S]*?)```/g, (_, lang, code) => {
    const highlighted = highlightCode(code, lang || 'text');
    return `<pre class="help-code" data-lang="${lang || 'text'}"><code>${highlighted}</code></pre>`;
  });

  // Inline code (`code`)
  result = result.replace(/`([^`]+)`/g, '<code class="help-inline-code">$1</code>');
  // Bold (**text**)
  result = result.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  // Line breaks
  result = result.replace(/\n/g, '<br>');

  return result;
}

// Hover provider for showing diagnostic help in Monaco
let hoverProviderDisposable: monaco.IDisposable | null = null;

// Find diagnostic at a given position
function findDiagnosticAtPosition(line: number, col: number): LintDiagnostic | null {
  if (!lintResult.value?.diagnostics) return null;

  for (const diag of lintResult.value.diagnostics) {
    const startLine = diag.location.start.line;
    const startCol = diag.location.start.column;
    const endLine = diag.location.end?.line ?? startLine;
    const endCol = diag.location.end?.column ?? startCol + 1;

    // Check if position is within diagnostic range
    if (line > startLine && line < endLine) {
      return diag;
    }
    if (line === startLine && line === endLine) {
      if (col >= startCol && col <= endCol) {
        return diag;
      }
    }
    if (line === startLine && line < endLine && col >= startCol) {
      return diag;
    }
    if (line === endLine && line > startLine && col <= endCol) {
      return diag;
    }
  }
  return null;
}

function registerHoverProvider() {
  if (hoverProviderDisposable) {
    hoverProviderDisposable.dispose();
  }

  hoverProviderDisposable = monaco.languages.registerHoverProvider('vue', {
    provideHover(model, position) {
      const contents: monaco.IMarkdownString[] = [];

      // Check if hovering over a diagnostic
      const diag = findDiagnosticAtPosition(position.lineNumber, position.column);
      if (diag) {
        // Add diagnostic message with severity indicator
        const severityLabel = diag.severity === 'error' ? 'Error' : 'Warning';
        contents.push({
          value: `**[${severityLabel}]** \`${diag.rule}\`\n\n${diag.message}`,
        });

        // Add help if available (render as markdown)
        if (diag.help) {
          contents.push({
            value: `---\n**Hint**\n\n${diag.help}`,
          });
        }
      }

      if (contents.length === 0) return null;

      return {
        contents,
      };
    },
  });
}

function getSeverityIcon(severity: 'error' | 'warning'): string {
  return severity === 'error' ? '✕' : '⚠';
}

function getSeverityClass(severity: 'error' | 'warning'): string {
  return severity === 'error' ? 'severity-error' : 'severity-warning';
}

let lintTimer: ReturnType<typeof setTimeout> | null = null;

watch(
  source,
  () => {
    if (lintTimer) clearTimeout(lintTimer);
    lintTimer = setTimeout(lint, 300);
  },
  { immediate: true }
);

// Workaround for vite-plugin-vize prop reactivity issue
// Use getWasm() directly instead of props since prop updates aren't detected
let hasCompilerInitialized = false;
let pollInterval: ReturnType<typeof setInterval> | null = null;

function tryInitialize() {
  const compiler = getWasm();
  if (compiler && !hasCompilerInitialized) {
    hasCompilerInitialized = true;
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
    lint();
    loadRules();
  }
}

onMounted(() => {
  loadLocaleConfig();
  loadRuleConfig();
  registerHoverProvider();

  // Try to initialize immediately if compiler is available
  tryInitialize();

  // If not, poll for it
  if (!hasCompilerInitialized) {
    pollInterval = setInterval(tryInitialize, 100);
    // Stop polling after 10 seconds
    setTimeout(() => {
      if (pollInterval) {
        clearInterval(pollInterval);
        pollInterval = null;
      }
    }, 10000);
  }
});

onUnmounted(() => {
  if (pollInterval) {
    clearInterval(pollInterval);
    pollInterval = null;
  }
  if (hoverProviderDisposable) {
    hoverProviderDisposable.dispose();
    hoverProviderDisposable = null;
  }
});
</script>

<template>
  <div class="patina-playground">
    <div class="panel input-panel">
      <div class="panel-header">
        <div class="header-title">
          <span class="icon">&#x26A0;</span>
          <h2>Source</h2>
        </div>
        <div class="panel-actions">
          <button @click="source = LINT_PRESET" class="btn-ghost">Reset</button>
        </div>
      </div>
      <div class="editor-container">
        <MonacoEditor ref="editorRef" v-model="source" language="vue" :diagnostics="diagnostics" />
      </div>
    </div>

    <div class="panel output-panel">
      <div class="panel-header">
        <div class="header-title">
          <span class="icon">&#x2714;</span>
          <h2>Lint Analysis</h2>
          <span v-if="lintTime !== null" class="perf-badge">
            {{ lintTime.toFixed(2) }}ms
          </span>
          <template v-if="lintResult">
            <span v-if="errorCount > 0" class="count-badge errors">{{ errorCount }}</span>
            <span v-if="warningCount > 0" class="count-badge warnings">{{ warningCount }}</span>
          </template>
        </div>
        <div class="tabs">
          <button
            :class="['tab', { active: activeTab === 'diagnostics' }]"
            @click="activeTab = 'diagnostics'"
          >Diagnostics
            <span v-if="lintResult?.diagnostics.length" class="tab-badge">{{ lintResult.diagnostics.length }}</span>
          </button>
          <button
            :class="['tab', { active: activeTab === 'rules' }]"
            @click="activeTab = 'rules'"
          >Rules
            <span class="tab-count">{{ enabledRuleCount }}/{{ rules.length }}</span>
          </button>
        </div>
      </div>

      <div class="output-content">
        <div v-if="error" class="error-panel">
          <div class="error-header">Lint Error</div>
          <pre class="error-content">{{ error }}</pre>
        </div>

        <template v-else-if="lintResult">
          <!-- Diagnostics Tab -->
          <div v-if="activeTab === 'diagnostics'" class="diagnostics-output">
            <div class="output-header-bar">
              <span class="output-title">Issues</span>
              <div class="locale-selector">
                <select v-model="currentLocale" @change="setLocale(currentLocale)">
                  <option v-for="locale in locales" :key="locale.code" :value="locale.code">
                    {{ locale.name }}
                  </option>
                </select>
              </div>
            </div>

            <div v-if="lintResult.diagnostics.length === 0" class="success-state">
              <span class="success-icon">&#x2713;</span>
              <span>No issues found</span>
            </div>

            <div v-else class="diagnostics-list">
              <div
                v-for="(diagnostic, i) in lintResult.diagnostics"
                :key="i"
                :class="['diagnostic-item', `severity-${diagnostic.severity}`]"
              >
                <div class="diagnostic-header">
                  <span class="severity-icon">{{ getSeverityIcon(diagnostic.severity) }}</span>
                  <code class="rule-id">{{ diagnostic.rule }}</code>
                  <span class="location-badge">
                    {{ diagnostic.location.start.line }}:{{ diagnostic.location.start.column }}
                  </span>
                </div>
                <div class="diagnostic-message">{{ diagnostic.message }}</div>
                <div v-if="diagnostic.help" class="diagnostic-help">
                  <div class="help-header">
                    <span class="help-icon">?</span>
                    <span class="help-label">Hint</span>
                  </div>
                  <div class="help-content" v-html="formatHelp(diagnostic.help)"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- Rules Tab -->
          <div v-else-if="activeTab === 'rules'" class="rules-output">
            <div class="output-header-bar">
              <span class="output-title">Rule Configuration</span>
              <div class="rules-actions">
                <button @click="enableAllRules" class="btn-action">Enable All</button>
                <button @click="disableAllRules" class="btn-action">Disable All</button>
              </div>
            </div>

            <div class="rules-toolbar">
              <input
                v-model="searchQuery"
                type="text"
                placeholder="Search rules..."
                class="search-input"
              />
              <select v-model="selectedCategory" class="category-select">
                <option v-for="cat in categories" :key="cat" :value="cat">
                  {{ cat === 'all' ? 'All Categories' : cat }}
                </option>
              </select>
            </div>

            <!-- Category toggle headers when filtering by category -->
            <div
              v-if="selectedCategory !== 'all'"
              class="category-toggle"
            >
              <label class="toggle-label">
                <input
                  type="checkbox"
                  :checked="isCategoryFullyEnabled(selectedCategory)"
                  :indeterminate="isCategoryPartiallyEnabled(selectedCategory)"
                  @change="toggleCategory(selectedCategory, ($event.target as HTMLInputElement).checked)"
                  class="rule-checkbox"
                />
                <span class="category-label">{{ selectedCategory }}</span>
                <span class="category-count">{{ filteredRules.length }} rules</span>
              </label>
            </div>

            <div class="rules-list">
              <div
                v-for="rule in filteredRules"
                :key="rule.name"
                :class="['rule-item', { disabled: !enabledRules.has(rule.name) }]"
              >
                <div class="rule-main">
                  <label class="rule-toggle">
                    <input
                      type="checkbox"
                      :checked="enabledRules.has(rule.name)"
                      @change="toggleRule(rule.name)"
                      class="rule-checkbox"
                    />
                    <code class="rule-id">{{ rule.name }}</code>
                  </label>
                  <div class="rule-badges">
                    <span class="badge category-badge">{{ rule.category }}</span>
                    <span :class="['badge', 'severity-badge', rule.defaultSeverity]">
                      {{ rule.defaultSeverity }}
                    </span>
                    <span v-if="rule.fixable" class="badge fixable-badge">fix</span>
                  </div>
                </div>
                <div class="rule-description">{{ rule.description }}</div>
              </div>

              <div v-if="filteredRules.length === 0" class="empty-state">
                No rules match your search
              </div>
            </div>
          </div>
        </template>

        <div v-else class="loading-state">
          <span>Enter Vue code to see lint results</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.patina-playground {
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
  color: var(--accent-rust);
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

.tab-count {
  font-size: 0.625rem;
  color: var(--text-muted);
  font-family: 'JetBrains Mono', monospace;
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
  background: linear-gradient(135deg, rgba(245, 158, 11, 0.15), rgba(239, 68, 68, 0.15));
  border: 1px solid rgba(245, 158, 11, 0.3);
  border-radius: 4px;
  margin-bottom: 0.75rem;
}

.output-title {
  font-size: 0.75rem;
  font-weight: 600;
  color: #fbbf24;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.locale-selector select {
  padding: 0.25rem 0.5rem;
  font-size: 0.625rem;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  border-radius: 3px;
  color: var(--text-primary);
  cursor: pointer;
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

.rule-id {
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
  margin-top: 0.75rem;
  padding: 0.75rem;
  background: linear-gradient(135deg, rgba(96, 165, 250, 0.08) 0%, rgba(147, 51, 234, 0.05) 100%);
  border: 1px solid rgba(96, 165, 250, 0.2);
  border-radius: 6px;
  font-size: 0.85rem;
}

.help-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid rgba(96, 165, 250, 0.15);
}

.help-icon {
  font-size: 1rem;
}

.help-label {
  font-weight: 600;
  color: #60a5fa;
  font-size: 0.9rem;
}

.help-content {
  color: var(--text-primary);
  line-height: 1.6;
}

.help-content :deep(strong) {
  color: #f59e0b;
  font-weight: 600;
}

.help-content :deep(.help-code) {
  margin: 0.5rem 0;
  padding: 0.75rem;
  background: rgba(0, 0, 0, 0.3);
  border-radius: 4px;
  overflow-x: auto;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.8rem;
  line-height: 1.5;
}

.help-content :deep(.help-code code) {
  color: #a5d6ff;
  background: none;
  padding: 0;
}

.help-content :deep(.help-inline-code) {
  background: rgba(110, 118, 129, 0.3);
  padding: 0.15rem 0.4rem;
  border-radius: 3px;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.85em;
  color: #ff7b72;
}

/* Syntax highlighting colors */
.help-content :deep(.hl-keyword) {
  color: #ff7b72;
}

.help-content :deep(.hl-vue-api) {
  color: #7ee787;
}

.help-content :deep(.hl-string) {
  color: #a5d6ff;
}

.help-content :deep(.hl-comment) {
  color: #8b949e;
  font-style: italic;
}

.help-content :deep(.hl-tag) {
  color: #7ee787;
}

.help-content :deep(.hl-directive) {
  color: #d2a8ff;
}

.help-content :deep(.hl-delimiter) {
  color: #ffa657;
}

.help-content :deep(.hl-type) {
  color: #79c0ff;
}

.help-content :deep(.hl-number) {
  color: #79c0ff;
}

.help-content :deep(.hl-property) {
  color: #79c0ff;
}

.help-content :deep(.hl-value) {
  color: #a5d6ff;
}

/* Rules Output */
.rules-output {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.rules-toolbar {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}

.rules-actions {
  display: flex;
  gap: 0.375rem;
}

.btn-action {
  padding: 0.25rem 0.5rem;
  font-size: 0.625rem;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  border-radius: 3px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s;
}

.btn-action:hover {
  background: var(--bg-secondary);
  color: var(--text-primary);
  border-color: var(--accent-rust);
}

.search-input {
  flex: 1;
  padding: 0.375rem 0.625rem;
  font-size: 0.75rem;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  color: var(--text-primary);
}

.search-input::placeholder {
  color: var(--text-muted);
}

.search-input:focus {
  outline: none;
  border-color: var(--accent-rust);
}

.category-select {
  padding: 0.375rem 0.625rem;
  font-size: 0.75rem;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  color: var(--text-primary);
  cursor: pointer;
}

.category-select:focus {
  outline: none;
  border-color: var(--accent-rust);
}

.category-toggle {
  padding: 0.625rem 0.75rem;
  margin-bottom: 0.75rem;
  background: var(--bg-tertiary);
  border-radius: 4px;
  border: 1px solid var(--border-primary);
}

.toggle-label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}

.category-label {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--text-primary);
}

.category-count {
  font-size: 0.6875rem;
  color: var(--text-muted);
}

/* Rules List */
.rules-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.rule-item {
  padding: 0.625rem 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  transition: all 0.15s;
}

.rule-item:hover {
  border-color: var(--border-secondary);
}

.rule-item.disabled {
  opacity: 0.5;
}

.rule-main {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  margin-bottom: 0.25rem;
}

.rule-toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}

.rule-checkbox {
  width: 14px;
  height: 14px;
  accent-color: var(--accent-rust);
  cursor: pointer;
}

.rules-output .rule-id {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text-primary);
  background: transparent;
  padding: 0;
}

.rule-item.disabled .rule-id {
  text-decoration: line-through;
}

.rule-badges {
  display: flex;
  gap: 0.375rem;
  flex-wrap: wrap;
}

.badge {
  display: inline-block;
  padding: 0.0625rem 0.375rem;
  font-size: 0.5625rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border-radius: 2px;
}

.category-badge {
  background: var(--bg-tertiary);
  color: var(--text-muted);
}

.severity-badge {
  background: var(--bg-tertiary);
}

.severity-badge.error {
  background: rgba(239, 68, 68, 0.15);
  color: #f87171;
}

.severity-badge.warning {
  background: rgba(245, 158, 11, 0.15);
  color: #fbbf24;
}

.fixable-badge {
  background: rgba(74, 222, 128, 0.15);
  color: #4ade80;
}

.rule-description {
  font-size: 0.6875rem;
  color: var(--text-muted);
  line-height: 1.4;
  padding-left: 1.375rem;
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
  .patina-playground {
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

  .rules-toolbar {
    flex-direction: column;
  }

  .rule-main {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.375rem;
  }

  .rule-description {
    padding-left: 0;
  }
}
</style>
