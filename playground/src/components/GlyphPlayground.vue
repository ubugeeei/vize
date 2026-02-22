<script setup lang="ts">
import { ref, watch, computed, inject, toRaw, onMounted, onUnmounted, type ComputedRef } from "vue";
import MonacoEditor from "./MonacoEditor.vue";
import CodeHighlight from "./CodeHighlight.vue";
import type { WasmModule, FormatOptions, FormatResult } from "../wasm/index";
import { getWasm } from "../wasm/index";
import { GLYPH_PRESET } from "./presets/glyph";
import { mdiFileEdit, mdiAutoFix, mdiCheck } from "@mdi/js";

const props = defineProps<{
  compiler: WasmModule | null;
}>();
const _injectedTheme = inject<ComputedRef<"dark" | "light">>("theme");
const theme = computed<"dark" | "light">(() => _injectedTheme?.value ?? "light");

const source = ref(GLYPH_PRESET);
const formatResult = ref<FormatResult | null>(null);
const formatKey = ref(0);
const error = ref<string | null>(null);
const formatTime = ref<number | null>(null);
const activeTab = ref<"formatted" | "diff" | "options">("formatted");

// Format options
const options = ref<FormatOptions>({
  printWidth: 100,
  tabWidth: 2,
  useTabs: false,
  semi: true,
  singleQuote: false,
  jsxSingleQuote: false,
  trailingComma: "all",
  bracketSpacing: true,
  bracketSameLine: false,
  arrowParens: "always",
  endOfLine: "lf",
  quoteProps: "as-needed",
  singleAttributePerLine: false,
  vueIndentScriptAndStyle: false,
  sortAttributes: true,
  attributeSortOrder: "alphabetical",
  mergeBindAndNonBindAttrs: false,
  maxAttributesPerLine: null,
  normalizeDirectiveShorthands: true,
  sortBlocks: true,
});

const diffLines = computed(() => {
  if (!formatResult.value) return [];

  const original = source.value.split("\n");
  const formatted = formatResult.value.code.split("\n");
  const diff: Array<{ type: "same" | "removed" | "added"; content: string; lineNum: number }> = [];

  // Simple diff - just show removed and added lines
  const maxLen = Math.max(original.length, formatted.length);
  let origIdx = 0;
  let fmtIdx = 0;

  while (origIdx < original.length || fmtIdx < formatted.length) {
    const origLine = original[origIdx];
    const fmtLine = formatted[fmtIdx];

    if (origLine === fmtLine) {
      diff.push({ type: "same", content: origLine || "", lineNum: origIdx + 1 });
      origIdx++;
      fmtIdx++;
    } else if (origLine !== undefined && fmtLine !== undefined) {
      diff.push({ type: "removed", content: origLine, lineNum: origIdx + 1 });
      diff.push({ type: "added", content: fmtLine, lineNum: fmtIdx + 1 });
      origIdx++;
      fmtIdx++;
    } else if (origLine !== undefined) {
      diff.push({ type: "removed", content: origLine, lineNum: origIdx + 1 });
      origIdx++;
    } else if (fmtLine !== undefined) {
      diff.push({ type: "added", content: fmtLine, lineNum: fmtIdx + 1 });
      fmtIdx++;
    }
  }

  return diff;
});

async function format() {
  const compiler = getWasm();
  if (!compiler) return;

  const startTime = performance.now();
  error.value = null;

  try {
    const raw = toRaw(options.value);
    const opts: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(raw)) {
      if (v != null) opts[k] = v;
    }
    const result = compiler.formatSfc(source.value, opts);
    formatResult.value = result;
    formatKey.value++;
    formatTime.value = performance.now() - startTime;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    formatResult.value = null;
  }
}

function copyToClipboard(text: string) {
  navigator.clipboard.writeText(text);
}

let formatTimer: ReturnType<typeof setTimeout> | null = null;

watch(
  [source, options],
  () => {
    if (!getWasm()) return;
    if (formatTimer) clearTimeout(formatTimer);
    formatTimer = setTimeout(format, 300);
  },
  { deep: true },
);

let hasInitialized = false;
let pollInterval: ReturnType<typeof setInterval> | null = null;

function tryInitialize() {
  const compiler = getWasm();
  if (compiler && !hasInitialized) {
    hasInitialized = true;
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
    format();
  }
}

onMounted(() => {
  tryInitialize();
  if (!hasInitialized) {
    pollInterval = setInterval(tryInitialize, 100);
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
});
</script>

<template>
  <div class="glyph-playground">
    <div class="panel input-panel">
      <div class="panel-header">
        <div class="header-title">
          <svg class="icon" viewBox="0 0 24 24"><path :d="mdiFileEdit" fill="currentColor" /></svg>
          <h2>Source</h2>
        </div>
        <div class="panel-actions">
          <button @click="source = GLYPH_PRESET" class="btn-ghost">Reset</button>
          <button @click="copyToClipboard(source)" class="btn-ghost">Copy</button>
        </div>
      </div>
      <div class="editor-container">
        <MonacoEditor v-model="source" language="html" :theme="theme" />
      </div>
    </div>

    <div class="panel output-panel">
      <div class="panel-header">
        <div class="header-title">
          <svg class="icon" viewBox="0 0 24 24"><path :d="mdiAutoFix" fill="currentColor" /></svg>
          <h2>Code Formatting</h2>
          <span v-if="formatTime !== null" class="perf-badge"> {{ formatTime.toFixed(2) }}ms </span>
          <span
            v-if="formatResult"
            :class="['status-badge', formatResult.changed ? 'changed' : 'unchanged']"
          >
            {{ formatResult.changed ? "Changed" : "Unchanged" }}
          </span>
        </div>
        <div class="tabs">
          <button
            :class="['tab', { active: activeTab === 'formatted' }]"
            @click="activeTab = 'formatted'"
          >
            Formatted
          </button>
          <button :class="['tab', { active: activeTab === 'diff' }]" @click="activeTab = 'diff'">
            Diff
          </button>
          <button
            :class="['tab', { active: activeTab === 'options' }]"
            @click="activeTab = 'options'"
          >
            Options
          </button>
        </div>
      </div>

      <div class="output-content">
        <div v-if="error" class="error-panel">
          <div class="error-header">Format Error</div>
          <pre class="error-content">{{ error }}</pre>
        </div>

        <template v-else-if="formatResult">
          <!-- Formatted Tab -->
          <div v-if="activeTab === 'formatted'" class="formatted-output">
            <div class="output-header-bar">
              <span class="output-title">Formatted Code</span>
              <div class="output-actions">
                <button @click="copyToClipboard(formatResult?.code || '')" class="btn-ghost">
                  Copy
                </button>
              </div>
            </div>
            <div class="code-container">
              <CodeHighlight
                :key="formatKey"
                :code="formatResult.code"
                language="html"
                show-line-numbers
                :theme="theme"
              />
            </div>
          </div>

          <!-- Diff Tab -->
          <div v-else-if="activeTab === 'diff'" class="diff-output">
            <div class="output-header-bar">
              <span class="output-title">Changes</span>
              <span class="diff-stats">
                <span class="stat additions"
                  >+{{ diffLines.filter((l) => l.type === "added").length }}</span
                >
                <span class="stat deletions"
                  >-{{ diffLines.filter((l) => l.type === "removed").length }}</span
                >
              </span>
            </div>
            <div v-if="!formatResult.changed" class="success-state">
              <svg class="success-icon" viewBox="0 0 24 24">
                <path :d="mdiCheck" fill="currentColor" />
              </svg>
              <span>No changes needed</span>
            </div>
            <div v-else class="diff-view">
              <div class="diff-line-numbers">
                <span v-for="(line, i) in diffLines" :key="i" class="diff-ln">{{ i + 1 }}</span>
              </div>
              <div class="diff-code">
                <div
                  v-for="(line, i) in diffLines"
                  :key="i"
                  :class="['diff-line', `diff-${line.type}`]"
                >
                  <span class="line-prefix">{{
                    line.type === "removed" ? "-" : line.type === "added" ? "+" : " "
                  }}</span>
                  <span class="line-content">{{ line.content || " " }}</span>
                </div>
              </div>
            </div>
          </div>

          <!-- Options Tab -->
          <div v-else-if="activeTab === 'options'" class="options-output">
            <div class="output-header-bar">
              <span class="output-title">Format Configuration</span>
            </div>
            <div class="options-content">
              <div class="options-section">
                <h4 class="section-title">Layout</h4>
                <div class="options-grid">
                  <label class="option-card">
                    <div class="option-header">
                      <span class="option-name">Print Width</span>
                      <input
                        type="number"
                        v-model.number="options.printWidth"
                        min="40"
                        max="200"
                        class="option-input"
                      />
                    </div>
                    <span class="option-desc">Maximum line length before wrapping</span>
                  </label>
                  <label class="option-card">
                    <div class="option-header">
                      <span class="option-name">Tab Width</span>
                      <input
                        type="number"
                        v-model.number="options.tabWidth"
                        min="1"
                        max="8"
                        class="option-input"
                      />
                    </div>
                    <span class="option-desc">Number of spaces per indentation level</span>
                  </label>
                </div>
              </div>

              <div class="options-section">
                <h4 class="section-title">Style</h4>
                <div class="toggle-grid">
                  <label class="toggle-card">
                    <div class="toggle-main">
                      <input type="checkbox" v-model="options.useTabs" class="toggle-checkbox" />
                      <span class="toggle-name">Use Tabs</span>
                    </div>
                    <span class="toggle-desc">Indent with tabs instead of spaces</span>
                  </label>
                  <label class="toggle-card">
                    <div class="toggle-main">
                      <input type="checkbox" v-model="options.semi" class="toggle-checkbox" />
                      <span class="toggle-name">Semicolons</span>
                    </div>
                    <span class="toggle-desc">Add semicolons at the end of statements</span>
                  </label>
                  <label class="toggle-card">
                    <div class="toggle-main">
                      <input
                        type="checkbox"
                        v-model="options.singleQuote"
                        class="toggle-checkbox"
                      />
                      <span class="toggle-name">Single Quotes</span>
                    </div>
                    <span class="toggle-desc">Use single quotes instead of double quotes</span>
                  </label>
                  <label class="toggle-card">
                    <div class="toggle-main">
                      <input
                        type="checkbox"
                        v-model="options.bracketSpacing"
                        class="toggle-checkbox"
                      />
                      <span class="toggle-name">Bracket Spacing</span>
                    </div>
                    <span class="toggle-desc"
                      >Print spaces between brackets in object literals</span
                    >
                  </label>
                  <label class="toggle-card">
                    <div class="toggle-main">
                      <input
                        type="checkbox"
                        v-model="options.bracketSameLine"
                        class="toggle-checkbox"
                      />
                      <span class="toggle-name">Bracket Same Line</span>
                    </div>
                    <span class="toggle-desc">Put closing bracket on the same line</span>
                  </label>
                  <label class="toggle-card">
                    <div class="toggle-main">
                      <input
                        type="checkbox"
                        v-model="options.jsxSingleQuote"
                        class="toggle-checkbox"
                      />
                      <span class="toggle-name">JSX Single Quotes</span>
                    </div>
                    <span class="toggle-desc">Use single quotes in JSX</span>
                  </label>
                </div>
              </div>

              <div class="options-section">
                <h4 class="section-title">Trailing / Parens</h4>
                <div class="options-grid">
                  <label class="option-card">
                    <div class="option-header">
                      <span class="option-name">Trailing Comma</span>
                      <select v-model="options.trailingComma" class="option-select">
                        <option value="all">All</option>
                        <option value="es5">ES5</option>
                        <option value="none">None</option>
                      </select>
                    </div>
                    <span class="option-desc">Print trailing commas wherever possible</span>
                  </label>
                  <label class="option-card">
                    <div class="option-header">
                      <span class="option-name">Arrow Parens</span>
                      <select v-model="options.arrowParens" class="option-select">
                        <option value="always">Always</option>
                        <option value="avoid">Avoid</option>
                      </select>
                    </div>
                    <span class="option-desc"
                      >Parentheses around a sole arrow function parameter</span
                    >
                  </label>
                  <label class="option-card">
                    <div class="option-header">
                      <span class="option-name">Quote Props</span>
                      <select v-model="options.quoteProps" class="option-select">
                        <option value="as-needed">As Needed</option>
                        <option value="consistent">Consistent</option>
                        <option value="preserve">Preserve</option>
                      </select>
                    </div>
                    <span class="option-desc">When to quote object properties</span>
                  </label>
                  <label class="option-card">
                    <div class="option-header">
                      <span class="option-name">End of Line</span>
                      <select v-model="options.endOfLine" class="option-select">
                        <option value="lf">LF</option>
                        <option value="crlf">CRLF</option>
                        <option value="cr">CR</option>
                        <option value="auto">Auto</option>
                      </select>
                    </div>
                    <span class="option-desc">End of line style</span>
                  </label>
                </div>
              </div>

              <div class="options-section">
                <h4 class="section-title">Vue SFC</h4>
                <div class="toggle-grid" style="margin-bottom: 0.75rem">
                  <label class="toggle-card">
                    <div class="toggle-main">
                      <input type="checkbox" v-model="options.sortBlocks" class="toggle-checkbox" />
                      <span class="toggle-name">Sort Blocks</span>
                    </div>
                    <span class="toggle-desc"
                      >Reorder blocks: script → setup → template → style</span
                    >
                  </label>
                </div>
                <div class="options-grid">
                  <label class="option-card">
                    <div class="option-header">
                      <span class="option-name">Attribute Sort Order</span>
                      <select v-model="options.attributeSortOrder" class="option-select">
                        <option value="alphabetical">Alphabetical</option>
                        <option value="as-written">As Written</option>
                      </select>
                    </div>
                    <span class="option-desc">How to sort attributes within priority groups</span>
                  </label>
                  <label class="option-card">
                    <div class="option-header">
                      <span class="option-name">Max Attrs Per Line</span>
                      <input
                        type="number"
                        :value="options.maxAttributesPerLine ?? ''"
                        @input="
                          options.maxAttributesPerLine =
                            ($event.target as HTMLInputElement).value === ''
                              ? null
                              : Number(($event.target as HTMLInputElement).value)
                        "
                        min="1"
                        max="20"
                        placeholder="auto"
                        class="option-input"
                      />
                    </div>
                    <span class="option-desc">Max attributes per line before wrapping</span>
                  </label>
                </div>
                <div class="toggle-grid" style="margin-top: 0.75rem">
                  <label class="toggle-card">
                    <div class="toggle-main">
                      <input
                        type="checkbox"
                        v-model="options.sortAttributes"
                        class="toggle-checkbox"
                      />
                      <span class="toggle-name">Sort Attributes</span>
                    </div>
                    <span class="toggle-desc">Sort HTML attributes in template</span>
                  </label>
                  <label class="toggle-card">
                    <div class="toggle-main">
                      <input
                        type="checkbox"
                        v-model="options.normalizeDirectiveShorthands"
                        class="toggle-checkbox"
                      />
                      <span class="toggle-name">Normalize Directives</span>
                    </div>
                    <span class="toggle-desc">Normalize v-bind/v-on/v-slot to shorthand</span>
                  </label>
                  <label class="toggle-card">
                    <div class="toggle-main">
                      <input
                        type="checkbox"
                        v-model="options.singleAttributePerLine"
                        class="toggle-checkbox"
                      />
                      <span class="toggle-name">Single Attribute Per Line</span>
                    </div>
                    <span class="toggle-desc">Enforce single attribute per line in templates</span>
                  </label>
                  <label class="toggle-card">
                    <div class="toggle-main">
                      <input
                        type="checkbox"
                        v-model="options.vueIndentScriptAndStyle"
                        class="toggle-checkbox"
                      />
                      <span class="toggle-name">Indent Script/Style</span>
                    </div>
                    <span class="toggle-desc">Indent code inside script and style tags</span>
                  </label>
                  <label class="toggle-card">
                    <div class="toggle-main">
                      <input
                        type="checkbox"
                        v-model="options.mergeBindAndNonBindAttrs"
                        class="toggle-checkbox"
                      />
                      <span class="toggle-name">Merge Bind Attrs</span>
                    </div>
                    <span class="toggle-desc">Merge bind and non-bind attrs for sorting</span>
                  </label>
                </div>
              </div>
            </div>
          </div>
        </template>

        <div v-else class="loading-state">
          <span>Enter Vue code to format</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.glyph-playground {
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
  width: 1rem;
  height: 1rem;
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
  background: var(--color-success-bg);
  color: var(--color-success);
  border-radius: 3px;
  font-family: "JetBrains Mono", monospace;
}

.status-badge {
  font-size: 0.625rem;
  padding: 0.125rem 0.5rem;
  border-radius: 3px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.status-badge.changed {
  background: var(--color-warning-bg);
  color: var(--color-warning);
}

.status-badge.unchanged {
  background: var(--color-success-bg);
  color: var(--color-success);
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
  background: var(--color-error-bg);
  border: 1px solid var(--color-error-border);
  border-radius: 6px;
  overflow: hidden;
}

.error-header {
  padding: 0.5rem 0.75rem;
  background: var(--color-error-bg);
  color: var(--color-error);
  font-size: 0.75rem;
  font-weight: 600;
}

.error-content {
  padding: 0.75rem;
  font-size: 0.75rem;
  color: var(--color-error);
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
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  border-radius: 4px 4px 0 0;
  border-bottom: none;
}

.output-title {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--accent-rust);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.output-actions {
  display: flex;
  gap: 0.5rem;
}

/* Formatted Output */
.formatted-output {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.code-container {
  flex: 1;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 0 0 4px 4px;
  overflow: auto;
}

/* Diff Output */
.diff-output {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.diff-stats {
  display: flex;
  gap: 0.5rem;
}

.diff-stats .stat {
  font-size: 0.625rem;
  font-family: "JetBrains Mono", monospace;
  padding: 0.125rem 0.375rem;
  border-radius: 3px;
}

.diff-stats .additions {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.diff-stats .deletions {
  background: var(--color-error-bg);
  color: var(--color-error);
}

.success-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 2rem;
  color: var(--color-success);
  font-size: 0.875rem;
}

.success-icon {
  width: 1.25rem;
  height: 1.25rem;
}

.diff-view {
  flex: 1;
  display: flex;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 0 0 4px 4px;
  font-size: 0.8125rem;
  font-family: "JetBrains Mono", monospace;
  overflow: auto;
  line-height: 1.6;
}

.diff-line-numbers {
  display: flex;
  flex-direction: column;
  padding: 0.75rem 0;
  background: rgba(0, 0, 0, 0.15);
  border-right: 1px solid var(--border-primary);
  user-select: none;
  flex-shrink: 0;
}

.diff-ln {
  padding: 0 0.75rem;
  text-align: right;
  color: var(--text-muted);
  font-size: 0.6875rem;
  min-width: 2.5rem;
  opacity: 0.6;
}

.diff-code {
  flex: 1;
  padding: 0.75rem 0;
  overflow-x: auto;
}

.diff-line {
  display: flex;
  white-space: pre;
  padding: 0 1rem;
  min-height: 1.3em;
}

.diff-same {
  color: var(--text-secondary);
}

.diff-removed {
  background: var(--color-error-bg);
  color: var(--color-error);
}

.diff-added {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.line-prefix {
  width: 1.5rem;
  flex-shrink: 0;
  user-select: none;
  color: var(--text-muted);
}

.diff-removed .line-prefix {
  color: var(--color-error);
}

.diff-added .line-prefix {
  color: var(--color-success);
}

.line-content {
  flex: 1;
}

/* Options Output */
.options-output {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.options-content {
  flex: 1;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 0 0 4px 4px;
  padding: 1rem;
  overflow-y: auto;
}

.options-section {
  margin-bottom: 1.5rem;
}

.options-section:last-child {
  margin-bottom: 0;
}

.section-title {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  margin-bottom: 0.75rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--border-primary);
}

.options-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 0.75rem;
}

.option-card {
  padding: 0.75rem;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
}

.option-card:hover {
  border-color: var(--accent-rust);
}

.option-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.25rem;
}

.option-name {
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-primary);
}

.option-input {
  width: 60px;
  padding: 0.25rem 0.5rem;
  font-size: 0.75rem;
  background: var(--bg-primary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  color: var(--text-primary);
  font-family: "JetBrains Mono", monospace;
}

.option-input:focus {
  outline: none;
  border-color: var(--accent-rust);
}

.option-select {
  padding: 0.25rem 0.5rem;
  font-size: 0.75rem;
  background: var(--bg-primary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  color: var(--text-primary);
  font-family: "JetBrains Mono", monospace;
  cursor: pointer;
}

.option-select:focus {
  outline: none;
  border-color: var(--accent-rust);
}

.option-desc {
  font-size: 0.6875rem;
  color: var(--text-muted);
}

.toggle-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 0.5rem;
}

.toggle-card {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.625rem 0.75rem;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
}

.toggle-card:hover {
  border-color: var(--accent-rust);
}

.toggle-main {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.toggle-checkbox {
  width: 16px;
  height: 16px;
  accent-color: var(--accent-rust);
  cursor: pointer;
}

.toggle-name {
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-primary);
}

.toggle-desc {
  font-size: 0.6875rem;
  color: var(--text-muted);
  padding-left: 1.5rem;
}

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
  .glyph-playground {
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

  .options-grid,
  .toggle-grid {
    grid-template-columns: 1fr;
  }
}
</style>
