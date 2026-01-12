<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import MonacoEditor from './MonacoEditor.vue';
import CodeHighlight from './CodeHighlight.vue';
import type { WasmModule, ArtDescriptor, CsfOutput } from '../wasm/index';

const props = defineProps<{
  compiler: WasmModule | null;
}>();

const ART_PRESET = `<art
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

<script setup lang="ts">
import Button from './Button.vue'
<\/script>
`;

const source = ref(ART_PRESET);
const parsedArt = ref<ArtDescriptor | null>(null);
const csfOutput = ref<CsfOutput | null>(null);
const error = ref<string | null>(null);
const activeTab = ref<'parsed' | 'csf' | 'variants'>('parsed');
const compileTime = ref<number | null>(null);

const variantCount = computed(() => parsedArt.value?.variants.length ?? 0);

// Design tokens extraction
interface DesignToken {
  name: string;
  value: string;
  type: 'color' | 'size' | 'other';
}

const designTokens = computed((): DesignToken[] => {
  if (!parsedArt.value) return [];

  const tokens: DesignToken[] = [];
  const cssVarRegex = /--([a-zA-Z0-9-]+)\s*:\s*([^;]+)/g;

  // Extract from styles array if available
  const styles = parsedArt.value.styles || [];
  for (const style of styles) {
    const content = style.content || '';
    let match;
    while ((match = cssVarRegex.exec(content)) !== null) {
      const name = `--${match[1]}`;
      const value = match[2].trim();
      tokens.push({
        name,
        value,
        type: isColorValue(value) ? 'color' : isSizeValue(value) ? 'size' : 'other',
      });
    }
  }

  return tokens;
});

const colorTokens = computed(() => designTokens.value.filter(t => t.type === 'color'));
const sizeTokens = computed(() => designTokens.value.filter(t => t.type === 'size'));
const otherTokens = computed(() => designTokens.value.filter(t => t.type === 'other'));

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

  try {
    // Parse Art file
    const parsed = props.compiler.parseArt(source.value, {
      filename: 'example.art.vue',
    });
    parsedArt.value = parsed;

    // Transform to CSF
    const csf = props.compiler.artToCsf(source.value, {
      filename: 'example.art.vue',
    });
    csfOutput.value = csf;

    compileTime.value = performance.now() - startTime;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    parsedArt.value = null;
    csfOutput.value = null;
  }
}

let compileTimer: ReturnType<typeof setTimeout> | null = null;

watch(
  source,
  () => {
    if (compileTimer) clearTimeout(compileTimer);
    compileTimer = setTimeout(compile, 300);
  },
  { immediate: true }
);

watch(
  () => props.compiler,
  () => {
    if (props.compiler) compile();
  }
);
</script>

<template>
  <div class="musea-playground">
    <div class="panel input-panel">
      <div class="panel-header">
        <h2>Art File (.art.vue)</h2>
        <div class="panel-actions">
          <button @click="source = ART_PRESET" class="btn-ghost">Reset</button>
        </div>
      </div>
      <div class="editor-container">
        <MonacoEditor v-model="source" language="vue" />
      </div>
    </div>

    <div class="panel output-panel">
      <div class="panel-header">
        <h2>
          Output
          <span v-if="compileTime !== null" class="compile-time">
            {{ compileTime.toFixed(4) }}ms
          </span>
        </h2>
        <div class="tabs">
          <button
            :class="['tab', { active: activeTab === 'parsed' }]"
            @click="activeTab = 'parsed'"
          >
            Parsed
          </button>
          <button
            :class="['tab', { active: activeTab === 'variants' }]"
            @click="activeTab = 'variants'"
          >
            Variants ({{ variantCount }})
          </button>
          <button
            :class="['tab', { active: activeTab === 'csf' }]"
            @click="activeTab = 'csf'"
          >
            Storybook CSF
          </button>
        </div>
      </div>

      <div class="output-content">
        <div v-if="error" class="error">
          <h3>Parse Error</h3>
          <pre>{{ error }}</pre>
        </div>

        <template v-else-if="parsedArt">
          <!-- Parsed Tab -->
          <div v-if="activeTab === 'parsed'" class="parsed-output">
            <h4>Metadata</h4>
            <div class="metadata-grid">
              <div class="metadata-item">
                <span class="label">Title</span>
                <span class="value">{{ parsedArt.metadata.title }}</span>
              </div>
              <div v-if="parsedArt.metadata.description" class="metadata-item">
                <span class="label">Description</span>
                <span class="value">{{ parsedArt.metadata.description }}</span>
              </div>
              <div v-if="parsedArt.metadata.component" class="metadata-item">
                <span class="label">Component</span>
                <span class="value">{{ parsedArt.metadata.component }}</span>
              </div>
              <div v-if="parsedArt.metadata.category" class="metadata-item">
                <span class="label">Category</span>
                <span class="value">{{ parsedArt.metadata.category }}</span>
              </div>
              <div v-if="parsedArt.metadata.tags.length" class="metadata-item">
                <span class="label">Tags</span>
                <span class="value">
                  <span
                    v-for="tag in parsedArt.metadata.tags"
                    :key="tag"
                    class="tag"
                  >
                    {{ tag }}
                  </span>
                </span>
              </div>
              <div class="metadata-item">
                <span class="label">Status</span>
                <span :class="['value', 'status', parsedArt.metadata.status]">
                  {{ parsedArt.metadata.status }}
                </span>
              </div>
            </div>

            <h4>File Info</h4>
            <div class="file-info">
              <span v-if="parsedArt.hasScriptSetup" class="badge">Script Setup</span>
              <span v-if="parsedArt.hasScript" class="badge">Script</span>
              <span v-if="parsedArt.styleCount > 0" class="badge">
                {{ parsedArt.styleCount }} Style{{ parsedArt.styleCount > 1 ? 's' : '' }}
              </span>
            </div>

            <!-- Design Tokens -->
            <template v-if="designTokens.length > 0">
              <h4>Design Tokens ({{ designTokens.length }})</h4>

              <!-- Color Tokens -->
              <div v-if="colorTokens.length > 0" class="token-section">
                <h5>Colors</h5>
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
                      <span class="token-name">{{ token.name }}</span>
                      <span class="token-value">{{ token.value }}</span>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Size Tokens -->
              <div v-if="sizeTokens.length > 0" class="token-section">
                <h5>Sizes</h5>
                <div class="token-list">
                  <div
                    v-for="token in sizeTokens"
                    :key="token.name"
                    class="size-token"
                    @click="copyToClipboard(token.name)"
                    :title="`Click to copy: ${token.name}`"
                  >
                    <span class="token-name">{{ token.name }}</span>
                    <span class="token-value">{{ token.value }}</span>
                    <div class="size-preview" :style="{ width: token.value }"></div>
                  </div>
                </div>
              </div>

              <!-- Other Tokens -->
              <div v-if="otherTokens.length > 0" class="token-section">
                <h5>Other</h5>
                <div class="token-list">
                  <div
                    v-for="token in otherTokens"
                    :key="token.name"
                    class="other-token"
                    @click="copyToClipboard(token.name)"
                    :title="`Click to copy: ${token.name}`"
                  >
                    <span class="token-name">{{ token.name }}</span>
                    <span class="token-value">{{ token.value }}</span>
                  </div>
                </div>
              </div>
            </template>
          </div>

          <!-- Variants Tab -->
          <div v-else-if="activeTab === 'variants'" class="variants-output">
            <div
              v-for="variant in parsedArt.variants"
              :key="variant.name"
              class="variant-card"
            >
              <div class="variant-header">
                <h5>{{ variant.name }}</h5>
                <div class="variant-badges">
                  <span v-if="variant.isDefault" class="badge default">Default</span>
                  <span v-if="variant.skipVrt" class="badge skip">Skip VRT</span>
                </div>
              </div>
              <div class="variant-template">
                <CodeHighlight :code="variant.template" language="html" />
              </div>
            </div>
          </div>

          <!-- CSF Tab -->
          <div v-else-if="activeTab === 'csf' && csfOutput" class="csf-output">
            <div class="csf-header">
              <h4>{{ csfOutput.filename }}</h4>
            </div>
            <CodeHighlight
              :code="csfOutput.code"
              language="typescript"
              show-line-numbers
            />
          </div>
        </template>

        <div v-else class="loading">
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
}

.panel-header h2 {
  font-size: 0.875rem;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.compile-time {
  font-size: 0.75rem;
  font-weight: 400;
  color: var(--text-muted);
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

.parsed-output h4,
.variants-output h4,
.csf-output h4 {
  font-size: 0.875rem;
  font-weight: 600;
  margin-bottom: 0.75rem;
  color: var(--text-secondary);
}

.metadata-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 0.75rem;
  margin-bottom: 1.5rem;
}

.metadata-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 6px;
}

.metadata-item .label {
  font-size: 0.625rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
}

.metadata-item .value {
  font-size: 0.875rem;
  color: var(--text-primary);
}

.tag {
  display: inline-block;
  padding: 0.125rem 0.5rem;
  margin-right: 0.25rem;
  font-size: 0.75rem;
  background: var(--bg-tertiary);
  border-radius: 3px;
  color: var(--text-secondary);
}

.status {
  text-transform: capitalize;
}

.status.ready {
  color: #4ade80;
}

.status.draft {
  color: #fbbf24;
}

.status.deprecated {
  color: #f87171;
}

.file-info {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.badge {
  display: inline-block;
  padding: 0.25rem 0.5rem;
  font-size: 0.75rem;
  background: var(--bg-tertiary);
  border-radius: 4px;
  color: var(--text-secondary);
}

.badge.default {
  background: rgba(163, 72, 40, 0.2);
  color: var(--accent-rust);
}

.badge.skip {
  background: rgba(251, 191, 36, 0.2);
  color: #fbbf24;
}

.variant-card {
  margin-bottom: 1rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 8px;
  overflow: hidden;
}

.variant-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid var(--border-primary);
}

.variant-header h5 {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-primary);
}

.variant-badges {
  display: flex;
  gap: 0.5rem;
}

.variant-template {
  padding: 0.5rem;
}

.csf-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.75rem;
}

.csf-header h4 {
  font-family: 'JetBrains Mono', monospace;
  margin: 0;
}

.loading {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 200px;
  color: var(--text-muted);
}

/* Design Tokens */
.token-section {
  margin-top: 1rem;
  margin-bottom: 1rem;
}

.token-section h5 {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 0.5rem;
}

.color-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 0.75rem;
}

.color-token {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s;
}

.color-token:hover {
  border-color: var(--accent-rust);
  transform: translateY(-1px);
}

.color-swatch {
  width: 40px;
  height: 40px;
  border-radius: 6px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.1);
  flex-shrink: 0;
}

.token-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.token-name {
  font-size: 0.75rem;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.token-value {
  font-size: 0.625rem;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.token-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.size-token,
.other-token {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 6px;
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
  font-size: 0.8rem;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-primary);
}

.size-token .token-value,
.other-token .token-value {
  font-size: 0.75rem;
  font-family: 'JetBrains Mono', monospace;
  color: var(--accent-rust);
  min-width: 60px;
  text-align: right;
}

.size-preview {
  height: 8px;
  background: var(--accent-rust);
  border-radius: 4px;
  min-width: 4px;
  max-width: 200px;
}

/* Mobile responsive */
@media (max-width: 768px) {
  .musea-playground {
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

  .metadata-grid {
    grid-template-columns: 1fr;
  }
}
</style>
