<script setup lang="ts">
import { ref, watch, computed, onMounted } from 'vue';
import MonacoEditor from './MonacoEditor.vue';
import type { WasmModule, LintResult, LintDiagnostic, LintRule } from '../wasm/index';

interface Diagnostic {
  message: string;
  startLine: number;
  startColumn: number;
  endLine?: number;
  endColumn?: number;
  severity: 'error' | 'warning' | 'info';
}

const props = defineProps<{
  compiler: WasmModule | null;
}>();

const LINT_PRESET = `<template>
  <div class="container">
    <!-- This will trigger vue/require-v-for-key -->
    <ul>
      <li v-for="item in items">{{ item.name }}</li>
    </ul>

    <!-- This will trigger vue/no-use-v-if-with-v-for -->
    <div v-for="user in users" v-if="user.active" :key="user.id">
      {{ user.name }}
    </div>

    <!-- This is valid -->
    <template v-for="product in products" :key="product.id">
      <div v-if="product.inStock">
        {{ product.name }}
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
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
<\/script>

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

// Rule configuration state
const enabledRules = ref<Set<string>>(new Set());
const severityOverrides = ref<Map<string, 'error' | 'warning' | 'off'>>(new Map());
const STORAGE_KEY = 'vize-patina-rules-config';

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
const diagnostics = computed((): Diagnostic[] => {
  if (!lintResult.value?.diagnostics) return [];
  const offset = templateLineOffset.value;
  return lintResult.value.diagnostics.map(d => ({
    message: `[${d.rule}] ${d.message}`,
    startLine: d.location.start.line + offset,
    startColumn: d.location.start.column,
    endLine: (d.location.end?.line ?? d.location.start.line) + offset,
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
  if (!props.compiler) return;

  const startTime = performance.now();
  error.value = null;

  try {
    const result = props.compiler.lintSfc(source.value, {
      filename: 'example.vue',
      enabledRules: Array.from(enabledRules.value),
      severityOverrides: Object.fromEntries(severityOverrides.value),
    });
    lintResult.value = result;
    lintTime.value = performance.now() - startTime;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    lintResult.value = null;
  }
}

function loadRules() {
  if (!props.compiler) return;

  try {
    rules.value = props.compiler.getLintRules();
    initializeRuleState();
  } catch (e) {
    console.error('Failed to load rules:', e);
  }
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

watch(
  () => props.compiler,
  () => {
    if (props.compiler) {
      lint();
      loadRules();
    }
  }
);

onMounted(() => {
  loadRuleConfig();
  if (props.compiler) {
    loadRules();
  }
});
</script>

<template>
  <div class="patina-playground">
    <div class="panel input-panel">
      <div class="panel-header">
        <h2>Vue SFC (.vue)</h2>
        <div class="panel-actions">
          <button @click="source = LINT_PRESET" class="btn-ghost">Reset</button>
        </div>
      </div>
      <div class="editor-container">
        <MonacoEditor v-model="source" language="vue" :diagnostics="diagnostics" />
      </div>
    </div>

    <div class="panel output-panel">
      <div class="panel-header">
        <h2>
          Lint Results
          <span v-if="lintTime !== null" class="lint-time">
            {{ lintTime.toFixed(4) }}ms
          </span>
        </h2>
        <div class="summary" v-if="lintResult">
          <span :class="['count', { 'has-errors': errorCount > 0 }]">
            {{ errorCount }} error{{ errorCount !== 1 ? 's' : '' }}
          </span>
          <span :class="['count', { 'has-warnings': warningCount > 0 }]">
            {{ warningCount }} warning{{ warningCount !== 1 ? 's' : '' }}
          </span>
        </div>
        <div class="tabs">
          <button
            :class="['tab', { active: activeTab === 'diagnostics' }]"
            @click="activeTab = 'diagnostics'"
          >
            Diagnostics ({{ lintResult?.diagnostics.length ?? 0 }})
          </button>
          <button
            :class="['tab', { active: activeTab === 'rules' }]"
            @click="activeTab = 'rules'"
          >
            Rules ({{ rules.length }})
          </button>
        </div>
      </div>

      <div class="output-content">
        <div v-if="error" class="error">
          <h3>Lint Error</h3>
          <pre>{{ error }}</pre>
        </div>

        <template v-else-if="lintResult">
          <!-- Diagnostics Tab -->
          <div v-if="activeTab === 'diagnostics'" class="diagnostics-output">
            <div v-if="lintResult.diagnostics.length === 0" class="no-issues">
              <span class="check-icon">✓</span>
              <span>No issues found</span>
            </div>

            <div
              v-for="(diagnostic, i) in lintResult.diagnostics"
              :key="i"
              :class="['diagnostic-card', getSeverityClass(diagnostic.severity)]"
            >
              <div class="diagnostic-header">
                <span class="severity-icon">{{ getSeverityIcon(diagnostic.severity) }}</span>
                <span class="rule-name">{{ diagnostic.rule }}</span>
                <span class="location">
                  {{ diagnostic.location.start.line }}:{{ diagnostic.location.start.column }}
                </span>
              </div>
              <div class="diagnostic-message">{{ diagnostic.message }}</div>
              <div v-if="diagnostic.help" class="diagnostic-help">
                <span class="help-icon">💡</span>
                {{ diagnostic.help }}
              </div>
            </div>
          </div>

          <!-- Rules Tab -->
          <div v-else-if="activeTab === 'rules'" class="rules-output">
            <div class="rules-toolbar">
              <div class="rules-filters">
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
              <div class="rules-actions">
                <button @click="enableAllRules" class="btn-action">Enable All</button>
                <button @click="disableAllRules" class="btn-action">Disable All</button>
              </div>
            </div>

            <div class="rules-count">
              {{ enabledRuleCount }} enabled / {{ filteredRules.length }} of {{ rules.length }} rules
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
                <span class="category-name">{{ selectedCategory }}</span>
                <span class="category-count">({{ filteredRules.length }} rules)</span>
              </label>
            </div>

            <div
              v-for="rule in filteredRules"
              :key="rule.name"
              :class="['rule-card', { disabled: !enabledRules.has(rule.name) }]"
            >
              <div class="rule-header">
                <label class="rule-toggle">
                  <input
                    type="checkbox"
                    :checked="enabledRules.has(rule.name)"
                    @change="toggleRule(rule.name)"
                    class="rule-checkbox"
                  />
                  <span class="rule-name">{{ rule.name }}</span>
                </label>
                <div class="rule-badges">
                  <span class="badge category">{{ rule.category }}</span>
                  <span :class="['badge', 'severity', rule.defaultSeverity]">
                    {{ rule.defaultSeverity }}
                  </span>
                  <span v-if="rule.fixable" class="badge fixable">fixable</span>
                </div>
              </div>
              <div class="rule-description">{{ rule.description }}</div>
            </div>

            <div v-if="filteredRules.length === 0" class="no-rules">
              No rules match your search
            </div>
          </div>
        </template>

        <div v-else class="loading">
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
  grid-column: 1 / -1; /* Span full width of parent grid */
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
  flex-wrap: wrap;
  gap: 0.5rem;
}

.panel-header h2 {
  font-size: 0.875rem;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.lint-time {
  font-size: 0.75rem;
  font-weight: 400;
  color: var(--text-muted);
}

.summary {
  display: flex;
  gap: 0.75rem;
}

.count {
  font-size: 0.75rem;
  color: var(--text-muted);
}

.count.has-errors {
  color: #ef4444;
  font-weight: 600;
}

.count.has-warnings {
  color: #f59e0b;
  font-weight: 600;
}

.panel-actions {
  display: flex;
  gap: 0.5rem;
}

.btn-ghost {
  padding: 0.25rem 0.75rem;
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
  gap: 0.25rem;
}

.tab {
  padding: 0.375rem 0.75rem;
  font-size: 0.75rem;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s;
}

.tab:hover {
  color: var(--text-secondary);
  background: var(--bg-tertiary);
}

.tab.active {
  color: var(--accent-rust);
  background: rgba(163, 72, 40, 0.15);
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

.error {
  padding: 1rem;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
  border-radius: 6px;
}

.error h3 {
  color: #ef4444;
  margin-bottom: 0.5rem;
  font-size: 0.875rem;
}

.error pre {
  font-size: 0.75rem;
  color: #fca5a5;
  white-space: pre-wrap;
  word-break: break-word;
}

.no-issues {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 2rem;
  color: #4ade80;
  font-size: 1rem;
}

.check-icon {
  font-size: 1.5rem;
}

.diagnostic-card {
  margin-bottom: 0.75rem;
  padding: 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 6px;
  border-left: 3px solid;
}

.diagnostic-card.severity-error {
  border-left-color: #ef4444;
}

.diagnostic-card.severity-warning {
  border-left-color: #f59e0b;
}

.diagnostic-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
}

.severity-icon {
  font-size: 0.875rem;
  font-weight: bold;
}

.severity-error .severity-icon {
  color: #ef4444;
}

.severity-warning .severity-icon {
  color: #f59e0b;
}

.rule-name {
  font-size: 0.75rem;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-secondary);
}

.location {
  margin-left: auto;
  font-size: 0.75rem;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-muted);
}

.diagnostic-message {
  font-size: 0.875rem;
  color: var(--text-primary);
  margin-bottom: 0.5rem;
}

.diagnostic-help {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  font-size: 0.75rem;
  color: var(--text-muted);
  padding: 0.5rem;
  background: var(--bg-tertiary);
  border-radius: 4px;
}

.help-icon {
  flex-shrink: 0;
}

.rule-card {
  margin-bottom: 0.75rem;
  padding: 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 6px;
}

.rule-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
  flex-wrap: wrap;
}

.rules-output .rule-name {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-primary);
}

.rule-badges {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.badge {
  display: inline-block;
  padding: 0.125rem 0.5rem;
  font-size: 0.625rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border-radius: 3px;
}

.badge.category {
  background: var(--bg-tertiary);
  color: var(--text-secondary);
}

.badge.severity {
  background: var(--bg-tertiary);
}

.badge.severity.error {
  background: rgba(239, 68, 68, 0.2);
  color: #ef4444;
}

.badge.severity.warning {
  background: rgba(245, 158, 11, 0.2);
  color: #f59e0b;
}

.badge.fixable {
  background: rgba(74, 222, 128, 0.2);
  color: #4ade80;
}

.rule-description {
  font-size: 0.8rem;
  color: var(--text-secondary);
  line-height: 1.5;
}

.rules-toolbar {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  margin-bottom: 1rem;
}

.rules-filters {
  display: flex;
  gap: 0.5rem;
}

.rules-actions {
  display: flex;
  gap: 0.5rem;
}

.btn-action {
  padding: 0.375rem 0.75rem;
  font-size: 0.75rem;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s;
}

.btn-action:hover {
  background: var(--bg-secondary);
  color: var(--text-primary);
  border-color: var(--accent-rust);
}

.category-toggle {
  padding: 0.75rem;
  margin-bottom: 0.75rem;
  background: var(--bg-tertiary);
  border-radius: 6px;
  border: 1px solid var(--border-primary);
}

.toggle-label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}

.category-name {
  font-weight: 600;
  color: var(--text-primary);
}

.category-count {
  font-size: 0.75rem;
  color: var(--text-muted);
}

.rule-toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}

.rule-checkbox {
  width: 16px;
  height: 16px;
  accent-color: var(--accent-rust);
  cursor: pointer;
}

.rule-card.disabled {
  opacity: 0.5;
}

.rule-card.disabled .rule-name {
  text-decoration: line-through;
}

.search-input {
  flex: 1;
  padding: 0.5rem 0.75rem;
  font-size: 0.875rem;
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
  padding: 0.5rem 0.75rem;
  font-size: 0.875rem;
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

.rules-count {
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-bottom: 0.75rem;
}

.no-rules {
  text-align: center;
  padding: 2rem;
  color: var(--text-muted);
}

.loading {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 200px;
  color: var(--text-muted);
}

/* Mobile responsive */
@media (max-width: 768px) {
  .patina-playground {
    grid-template-columns: 1fr;
    grid-template-rows: 1fr 1fr;
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

  .summary {
    width: 100%;
    justify-content: flex-start;
  }

  .diagnostic-header {
    flex-wrap: wrap;
  }

  .rule-header {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
