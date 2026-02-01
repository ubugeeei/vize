<script setup lang="ts">
import { ref, watch, computed, onMounted } from "vue";
import MonacoEditor from "./MonacoEditor.vue";

interface Diagnostic {
  message: string;
  startLine: number;
  startColumn: number;
  endLine?: number;
  endColumn?: number;
  severity: "error" | "warning" | "info";
}
import CodeHighlight from "./CodeHighlight.vue";
import type { WasmModule, ArtDescriptor, CsfOutput } from "../wasm/index";

const props = defineProps<{
  compiler: WasmModule | null;
}>();

const ART_PRESET = `<script setup lang="ts">
import Button from './Button.vue'
<` + `/script>

<art
  title="Button"
  description="A versatile button component"
  component="./Button.vue"
  category="atoms"
  tags="ui,input"
>
  <variant name="Primary" default>
    <Button variant="primary">Click me</Button>
  </variant>

  <variant name="Secondary">
    <Button variant="secondary">Click me</Button>
  </variant>

  <variant name="With Icon">
    <Button variant="primary" icon="plus">Add Item</Button>
  </variant>

  <variant name="Disabled">
    <Button variant="primary" disabled>Disabled</Button>
  </variant>
</art>

<style>
:root {
  --color-primary: #3b82f6;
  --color-primary-hover: #2563eb;
  --color-secondary: #6b7280;
  --color-secondary-hover: #4b5563;
  --color-success: #10b981;
  --color-warning: #f59e0b;
  --color-error: #ef4444;
  --color-text: #1f2937;
  --color-text-muted: #6b7280;
  --color-background: #ffffff;
  --color-border: #e5e7eb;

  --spacing-xs: 4px;
  --spacing-sm: 8px;
  --spacing-md: 16px;
  --spacing-lg: 24px;
  --spacing-xl: 32px;

  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;

  --font-size-sm: 12px;
  --font-size-md: 14px;
  --font-size-lg: 16px;
}
<` + `/style>
`;

const source = ref(ART_PRESET);
const parsedArt = ref<ArtDescriptor | null>(null);
const csfOutput = ref<CsfOutput | null>(null);
const error = ref<string | null>(null);
const diagnostics = ref<Diagnostic[]>([]);
type TabType = "parsed" | "csf" | "variants";
const validTabs: TabType[] = ["parsed", "csf", "variants"];

function getTabFromUrl(): TabType {
  const params = new URLSearchParams(window.location.search);
  const tab = params.get("tab");
  if (tab && validTabs.includes(tab as TabType)) {
    return tab as TabType;
  }
  return "parsed";
}

function setTabToUrl(tab: TabType) {
  const url = new URL(window.location.href);
  url.searchParams.set("tab", tab);
  window.history.replaceState({}, "", url.toString());
}

const activeTab = ref<TabType>(getTabFromUrl());
const compileTime = ref<number | null>(null);

// Sync tab to URL
watch(activeTab, (tab) => {
  setTabToUrl(tab);
});

const variantCount = computed(() => parsedArt.value?.variants.length ?? 0);

// Design tokens extraction
interface DesignToken {
  name: string;
  value: string;
  type: "color" | "size" | "other";
}

const designTokens = computed((): DesignToken[] => {
  if (!parsedArt.value) return [];

  const tokens: DesignToken[] = [];
  const cssVarRegex = /--([a-zA-Z0-9-]+)\s*:\s*([^;]+)/g;

  // Try to extract from styles array first
  const styles = parsedArt.value.styles || [];
  let styleContent = "";

  if (styles.length > 0) {
    // Use styles from parsed result
    for (const style of styles) {
      styleContent += (style.content || "") + "\n";
    }
  } else {
    // Fallback: extract style content directly from source
    const styleRegex = new RegExp("<style[^>]*>([\\s\\S]*?)</style>", "g");
    let styleMatch;
    while ((styleMatch = styleRegex.exec(source.value)) !== null) {
      styleContent += styleMatch[1] + "\n";
    }
  }

  // Extract CSS variables from style content
  let match;
  while ((match = cssVarRegex.exec(styleContent)) !== null) {
    const name = `--${match[1]}`;
    const value = match[2].trim();
    tokens.push({
      name,
      value,
      type: isColorValue(value) ? "color" : isSizeValue(value) ? "size" : "other",
    });
  }

  return tokens;
});

const colorTokens = computed(() => designTokens.value.filter((t) => t.type === "color"));
const sizeTokens = computed(() => designTokens.value.filter((t) => t.type === "size"));
const otherTokens = computed(() => designTokens.value.filter((t) => t.type === "other"));

function isColorValue(value: string): boolean {
  return /^(#[0-9a-fA-F]{3,8}|rgb|rgba|hsl|hsla|transparent|currentColor|inherit)/i.test(value);
}

function isSizeValue(value: string): boolean {
  return /^-?\d+(\.\d+)?(px|rem|em|%|vh|vw|vmin|vmax|ch|ex)$/.test(value);
}

function copyToClipboard(text: string) {
  navigator.clipboard.writeText(text);
}

async function compile() {
  if (!props.compiler) return;

  const startTime = performance.now();
  error.value = null;
  diagnostics.value = [];

  try {
    // Parse Art file
    const parsed = props.compiler.parseArt(source.value, {
      filename: "example.art.vue",
    });
    parsedArt.value = parsed;

    // Transform to CSF
    const csf = props.compiler.artToCsf(source.value, {
      filename: "example.art.vue",
    });
    csfOutput.value = csf;

    compileTime.value = performance.now() - startTime;
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    error.value = message;
    parsedArt.value = null;
    csfOutput.value = null;

    // Parse line info from error message if available
    const lineMatch = message.match(/line\s*(\d+)/i);
    const colMatch = message.match(/col(?:umn)?\s*(\d+)/i);
    const line = lineMatch ? parseInt(lineMatch[1], 10) : 1;
    const col = colMatch ? parseInt(colMatch[1], 10) : 1;

    diagnostics.value = [
      {
        message,
        startLine: line,
        startColumn: col,
        severity: "error",
      },
    ];
  }
}

let compileTimer: ReturnType<typeof setTimeout> | null = null;

watch(
  source,
  () => {
    if (compileTimer) clearTimeout(compileTimer);
    compileTimer = setTimeout(compile, 300);
  },
  { immediate: true },
);

watch(
  () => props.compiler,
  () => {
    if (props.compiler) compile();
  },
);
</script>

<template>
  <div class="musea-playground">
    <div class="panel input-panel">
      <div class="panel-header">
        <div class="header-title">
          <span class="icon">&#x1F3A8;</span>
          <h2>Source</h2>
        </div>
        <div class="panel-actions">
          <button @click="source = ART_PRESET" class="btn-ghost">Reset</button>
        </div>
      </div>
      <div class="editor-container">
        <MonacoEditor v-model="source" language="vue" :diagnostics="diagnostics" />
      </div>
    </div>

    <div class="panel output-panel">
      <div class="panel-header">
        <div class="header-title">
          <span class="icon">&#x2756;</span>
          <h2>Art Analysis</h2>
          <span v-if="compileTime !== null" class="perf-badge">
            {{ compileTime.toFixed(2) }}ms
          </span>
        </div>
        <div class="tabs">
          <button
            :class="['tab', { active: activeTab === 'parsed' }]"
            @click="activeTab = 'parsed'"
          >
            Metadata
          </button>
          <button
            :class="['tab', { active: activeTab === 'variants' }]"
            @click="activeTab = 'variants'"
          >
            Variants
            <span v-if="variantCount > 0" class="tab-count">{{ variantCount }}</span>
          </button>
          <button :class="['tab', { active: activeTab === 'csf' }]" @click="activeTab = 'csf'">
            CSF
          </button>
        </div>
      </div>

      <div class="output-content">
        <div v-if="error" class="error-panel">
          <div class="error-header">Parse Error</div>
          <pre class="error-content">{{ error }}</pre>
        </div>

        <template v-else-if="parsedArt">
          <!-- Parsed Tab -->
          <div v-if="activeTab === 'parsed'" class="parsed-output">
            <div class="output-header-bar">
              <span class="output-title">Component Metadata</span>
              <div class="file-badges">
                <span v-if="parsedArt.hasScriptSetup" class="file-badge">setup</span>
                <span v-if="parsedArt.hasScript" class="file-badge">script</span>
                <span v-if="parsedArt.styleCount > 0" class="file-badge"
                  >{{ parsedArt.styleCount }} style</span
                >
              </div>
            </div>

            <div class="metadata-section">
              <div class="metadata-grid">
                <div class="metadata-item">
                  <span class="meta-label">Title</span>
                  <span class="meta-value">{{ parsedArt.metadata.title }}</span>
                </div>
                <div v-if="parsedArt.metadata.description" class="metadata-item span-full">
                  <span class="meta-label">Description</span>
                  <span class="meta-value">{{ parsedArt.metadata.description }}</span>
                </div>
                <div v-if="parsedArt.metadata.component" class="metadata-item">
                  <span class="meta-label">Component</span>
                  <code class="meta-code">{{ parsedArt.metadata.component }}</code>
                </div>
                <div v-if="parsedArt.metadata.category" class="metadata-item">
                  <span class="meta-label">Category</span>
                  <span class="meta-value category-value">{{ parsedArt.metadata.category }}</span>
                </div>
                <div v-if="parsedArt.metadata.tags?.length" class="metadata-item">
                  <span class="meta-label">Tags</span>
                  <span class="tags-list">
                    <span v-for="tag in parsedArt.metadata.tags" :key="tag" class="tag-item">{{
                      tag
                    }}</span>
                  </span>
                </div>
                <div class="metadata-item">
                  <span class="meta-label">Status</span>
                  <span :class="['status-badge', parsedArt.metadata.status]">{{
                    parsedArt.metadata.status
                  }}</span>
                </div>
              </div>
            </div>

            <!-- Design Tokens -->
            <template v-if="designTokens.length > 0">
              <div class="section-header">
                <span class="section-title">Design Tokens</span>
                <span class="section-count">{{ designTokens.length }}</span>
              </div>

              <!-- Color Tokens -->
              <div v-if="colorTokens.length > 0" class="token-section">
                <div class="token-category">Colors</div>
                <div class="color-grid">
                  <div
                    v-for="token in colorTokens"
                    :key="token.name"
                    class="color-token"
                    @click="copyToClipboard(token.name)"
                    :title="`Click to copy: ${token.name}`"
                  >
                    <div class="color-swatch" :style="{ background: token.value }"></div>
                    <div class="token-info">
                      <code class="token-name">{{ token.name }}</code>
                      <span class="token-value">{{ token.value }}</span>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Size Tokens -->
              <div v-if="sizeTokens.length > 0" class="token-section">
                <div class="token-category">Sizes</div>
                <div class="token-list">
                  <div
                    v-for="token in sizeTokens"
                    :key="token.name"
                    class="size-token"
                    @click="copyToClipboard(token.name)"
                    :title="`Click to copy: ${token.name}`"
                  >
                    <code class="token-name">{{ token.name }}</code>
                    <span class="token-value">{{ token.value }}</span>
                    <div class="size-preview" :style="{ width: token.value }"></div>
                  </div>
                </div>
              </div>

              <!-- Other Tokens -->
              <div v-if="otherTokens.length > 0" class="token-section">
                <div class="token-category">Other</div>
                <div class="token-list">
                  <div
                    v-for="token in otherTokens"
                    :key="token.name"
                    class="other-token"
                    @click="copyToClipboard(token.name)"
                    :title="`Click to copy: ${token.name}`"
                  >
                    <code class="token-name">{{ token.name }}</code>
                    <span class="token-value">{{ token.value }}</span>
                  </div>
                </div>
              </div>
            </template>
          </div>

          <!-- Variants Tab -->
          <div v-else-if="activeTab === 'variants'" class="variants-output">
            <div class="output-header-bar">
              <span class="output-title">Variants</span>
              <span class="variant-count"
                >{{ parsedArt.variants.length }} variant{{
                  parsedArt.variants.length !== 1 ? "s" : ""
                }}</span
              >
            </div>

            <div class="variants-list">
              <div v-for="variant in parsedArt.variants" :key="variant.name" class="variant-item">
                <div class="variant-header">
                  <div class="variant-name">
                    {{ variant.name }}
                    <span v-if="variant.isDefault" class="default-badge">default</span>
                    <span v-if="variant.skipVrt" class="skip-badge">skip vrt</span>
                  </div>
                  <button @click="copyToClipboard(variant.template)" class="btn-copy">Copy</button>
                </div>
                <div class="variant-template">
                  <CodeHighlight :code="variant.template" language="html" />
                </div>
              </div>
            </div>
          </div>

          <!-- CSF Tab -->
          <div v-else-if="activeTab === 'csf' && csfOutput" class="csf-output">
            <div class="output-header-bar">
              <span class="output-title">Storybook CSF</span>
              <div class="csf-actions">
                <code class="filename-badge">{{ csfOutput.filename }}</code>
                <button @click="copyToClipboard(csfOutput.code)" class="btn-copy">Copy</button>
              </div>
            </div>
            <div class="code-container">
              <CodeHighlight :code="csfOutput.code" language="typescript" show-line-numbers />
            </div>
          </div>
        </template>

        <div v-else class="loading-state">
          <span>Enter an Art file to see the output</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.musea-playground {
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
  font-family: "JetBrains Mono", monospace;
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

.tab-count {
  font-size: 0.625rem;
  color: var(--text-muted);
  font-family: "JetBrains Mono", monospace;
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
  background: linear-gradient(135deg, rgba(168, 85, 247, 0.15), rgba(236, 72, 153, 0.15));
  border: 1px solid rgba(168, 85, 247, 0.3);
  border-radius: 4px;
  margin-bottom: 0.75rem;
}

.output-title {
  font-size: 0.75rem;
  font-weight: 600;
  color: #a78bfa;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.file-badges {
  display: flex;
  gap: 0.375rem;
}

.file-badge {
  font-size: 0.5625rem;
  padding: 0.125rem 0.375rem;
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-muted);
  border-radius: 2px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

/* Metadata Section */
.metadata-section {
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  padding: 1rem;
  margin-bottom: 1rem;
}

.metadata-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 0.75rem;
}

.metadata-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.metadata-item.span-full {
  grid-column: 1 / -1;
}

.meta-label {
  font-size: 0.5625rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
}

.meta-value {
  font-size: 0.8125rem;
  color: var(--text-primary);
}

.meta-code {
  font-size: 0.75rem;
  font-family: "JetBrains Mono", monospace;
  color: #60a5fa;
  background: var(--bg-tertiary);
  padding: 0.125rem 0.375rem;
  border-radius: 3px;
}

.category-value {
  color: #a78bfa;
}

.tags-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem;
}

.tag-item {
  font-size: 0.6875rem;
  padding: 0.125rem 0.375rem;
  background: var(--bg-tertiary);
  border-radius: 3px;
  color: var(--text-secondary);
}

.status-badge {
  font-size: 0.6875rem;
  padding: 0.125rem 0.5rem;
  border-radius: 3px;
  text-transform: capitalize;
}

.status-badge.ready {
  background: rgba(74, 222, 128, 0.15);
  color: #4ade80;
}

.status-badge.draft {
  background: rgba(251, 191, 36, 0.15);
  color: #fbbf24;
}

.status-badge.deprecated {
  background: rgba(248, 113, 113, 0.15);
  color: #f87171;
}

/* Section Header */
.section-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--border-primary);
}

.section-title {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
}

.section-count {
  font-size: 0.625rem;
  padding: 0.0625rem 0.375rem;
  background: var(--bg-tertiary);
  border-radius: 8px;
  color: var(--text-muted);
  font-family: "JetBrains Mono", monospace;
}

/* Design Tokens */
.token-section {
  margin-bottom: 1rem;
}

.token-category {
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--text-muted);
  margin-bottom: 0.5rem;
}

.color-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 0.5rem;
}

.color-token {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  padding: 0.5rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s;
}

.color-token:hover {
  border-color: var(--accent-rust);
}

.color-swatch {
  width: 32px;
  height: 32px;
  border-radius: 4px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.1);
  flex-shrink: 0;
}

.token-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.token-name {
  font-size: 0.6875rem;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.token-value {
  font-size: 0.5625rem;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.token-list {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.size-token,
.other-token {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 0.625rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s;
}

.size-token:hover,
.other-token:hover {
  border-color: var(--accent-rust);
}

.size-token .token-name,
.other-token .token-name {
  flex: 1;
  font-size: 0.75rem;
}

.size-token .token-value,
.other-token .token-value {
  font-size: 0.6875rem;
  color: var(--accent-rust);
  min-width: 50px;
  text-align: right;
}

.size-preview {
  height: 6px;
  background: var(--accent-rust);
  border-radius: 3px;
  min-width: 4px;
  max-width: 150px;
}

/* Variants Output */
.variants-output {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.variant-count {
  font-size: 0.625rem;
  color: var(--text-muted);
  font-family: "JetBrains Mono", monospace;
}

.variants-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.variant-item {
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  overflow: hidden;
}

.variant-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  background: var(--bg-tertiary);
  border-bottom: 1px solid var(--border-primary);
}

.variant-name {
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-primary);
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.default-badge {
  font-size: 0.5625rem;
  padding: 0.0625rem 0.375rem;
  background: rgba(163, 72, 40, 0.2);
  color: var(--accent-rust);
  border-radius: 2px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.skip-badge {
  font-size: 0.5625rem;
  padding: 0.0625rem 0.375rem;
  background: rgba(251, 191, 36, 0.2);
  color: #fbbf24;
  border-radius: 2px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.btn-copy {
  padding: 0.125rem 0.375rem;
  font-size: 0.625rem;
  background: transparent;
  border: 1px solid var(--border-primary);
  border-radius: 3px;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s;
}

.btn-copy:hover {
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.variant-template {
  padding: 0.5rem;
}

/* CSF Output */
.csf-output {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.csf-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.filename-badge {
  font-size: 0.625rem;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-muted);
  background: rgba(255, 255, 255, 0.1);
  padding: 0.125rem 0.375rem;
  border-radius: 3px;
}

.code-container {
  flex: 1;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  overflow: auto;
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
  .musea-playground {
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

  .metadata-grid {
    grid-template-columns: 1fr;
  }

  .color-grid {
    grid-template-columns: 1fr;
  }
}
</style>
