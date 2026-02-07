<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { usePalette } from '../composables/usePalette'
import { getPreviewUrl } from '../api'
import { sendMessage } from '../composables/usePostMessage'
import TextControl from './controls/TextControl.vue'
import NumberControl from './controls/NumberControl.vue'
import BooleanControl from './controls/BooleanControl.vue'
import RangeControl from './controls/RangeControl.vue'
import SelectControl from './controls/SelectControl.vue'
import ColorControl from './controls/ColorControl.vue'

const props = defineProps<{
  artPath: string
  defaultVariantName?: string
}>()

const { palette, loading, error, values, load, setValue, resetValues } = usePalette()

const iframeRef = ref<HTMLIFrameElement | null>(null)
const iframeReady = ref(false)
const slotContent = ref('')
const copiedUsage = ref(false)

const previewUrl = computed(() => {
  if (!props.defaultVariantName) return ''
  return getPreviewUrl(props.artPath, props.defaultVariantName)
})

watch(() => props.artPath, (path) => {
  if (path) load(path)
  iframeReady.value = false
  slotContent.value = ''
}, { immediate: true })

// Send props to iframe when values change
watch(values, (newValues) => {
  const iframe = iframeRef.value
  if (!iframe || !iframeReady.value) return
  sendMessage(iframe, 'musea:set-props', { props: newValues })
}, { deep: true })

// Send slots to iframe when slot content changes
watch(slotContent, (content) => {
  const iframe = iframeRef.value
  if (!iframe || !iframeReady.value) return
  sendMessage(iframe, 'musea:set-slots', { slots: { default: content } })
})

function onIframeLoad() {
  iframeReady.value = true
  // Send initial props if any
  const iframe = iframeRef.value
  if (!iframe) return
  if (Object.keys(values.value).length > 0) {
    sendMessage(iframe, 'musea:set-props', { props: values.value })
  }
}

function onResetValues() {
  resetValues()
  slotContent.value = ''
  const iframe = iframeRef.value
  if (!iframe || !iframeReady.value) return
  sendMessage(iframe, 'musea:set-props', { props: values.value })
  sendMessage(iframe, 'musea:set-slots', { slots: { default: '' } })
}

// Generate usage code
const usageCode = computed(() => {
  if (!palette.value) return ''
  const componentName = palette.value.title || 'Component'
  const propsEntries = Object.entries(values.value).filter(([, v]) => v !== undefined && v !== '')
  if (propsEntries.length === 0 && !slotContent.value) {
    return `<${componentName} />`
  }
  const propsStr = propsEntries.map(([k, v]) => {
    if (typeof v === 'boolean') return v ? ` ${k}` : ` :${k}="false"`
    if (typeof v === 'number') return ` :${k}="${v}"`
    return ` ${k}="${String(v)}"`
  }).join('')
  if (slotContent.value) {
    return `<${componentName}${propsStr}>\n  ${slotContent.value}\n</${componentName}>`
  }
  return `<${componentName}${propsStr} />`
})

async function copyUsage() {
  try {
    await navigator.clipboard.writeText(usageCode.value)
    copiedUsage.value = true
    setTimeout(() => { copiedUsage.value = false }, 2000)
  } catch {
    // fallback
  }
}

function getControlComponent(kind: string) {
  switch (kind) {
    case 'text': return TextControl
    case 'number': return NumberControl
    case 'boolean': return BooleanControl
    case 'range': return RangeControl
    case 'select':
    case 'radio': return SelectControl
    case 'color': return ColorControl
    default: return TextControl
  }
}
</script>

<template>
  <div class="props-panel">
    <div v-if="loading" class="props-loading">
      <div class="loading-spinner" />
      Loading props...
    </div>

    <div v-else-if="error" class="props-error">
      {{ error }}
    </div>

    <template v-else-if="palette && palette.controls.length > 0">
      <!-- Live Preview -->
      <div v-if="previewUrl" class="props-preview">
        <div class="props-preview-header">
          <span class="props-preview-label">Live Preview</span>
        </div>
        <div class="props-preview-frame">
          <iframe
            ref="iframeRef"
            :src="previewUrl"
            @load="onIframeLoad"
          />
        </div>
      </div>

      <div class="props-header">
        <h3 class="props-title">Props Controls</h3>
        <button class="props-reset" @click="onResetValues">
          Reset
        </button>
      </div>

      <div class="props-grid">
        <template v-for="group in palette.groups" :key="group">
          <div v-if="group" class="props-group-header">{{ group }}</div>
          <template v-for="control in palette.controls.filter(c => c.group === group)" :key="control.name">
            <component
              :is="getControlComponent(control.control)"
              :label="control.name"
              :description="control.description"
              :required="control.required"
              :options="control.options"
              :min="control.range?.min"
              :max="control.range?.max"
              :step="control.range?.step"
              :model-value="values[control.name]"
              @update:model-value="(v: unknown) => setValue(control.name, v)"
            />
          </template>
        </template>

        <template v-for="control in palette.controls.filter(c => !c.group)" :key="control.name">
          <component
            :is="getControlComponent(control.control)"
            :label="control.name"
            :description="control.description"
            :required="control.required"
            :options="control.options"
            :min="control.range?.min"
            :max="control.range?.max"
            :step="control.range?.step"
            :model-value="values[control.name]"
            @update:model-value="(v: unknown) => setValue(control.name, v)"
          />
        </template>
      </div>

      <!-- Slot Editor -->
      <div class="props-slot-editor">
        <div class="props-slot-header">Slot Content</div>
        <textarea
          v-model="slotContent"
          class="props-slot-textarea"
          placeholder="Enter slot content (HTML)..."
          rows="3"
        />
      </div>

      <!-- Usage Code -->
      <div class="props-usage">
        <div class="props-usage-header">
          <span>Usage</span>
          <button class="props-copy-btn" @click="copyUsage">
            <svg v-if="!copiedUsage" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
            </svg>
            <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12">
              <polyline points="20 6 9 17 4 12" />
            </svg>
            {{ copiedUsage ? 'Copied!' : 'Copy' }}
          </button>
        </div>
        <pre class="props-usage-code">{{ usageCode }}</pre>
      </div>

      <div class="props-json">
        <div class="props-json-header">Current Values</div>
        <pre class="props-json-code">{{ JSON.stringify(values, null, 2) }}</pre>
      </div>
    </template>

    <div v-else class="props-empty">
      <p>No props controls available for this component.</p>
      <p class="props-empty-hint">
        Add a <code>component</code> attribute to the <code>&lt;art&gt;</code> block to enable props analysis.
      </p>
    </div>
  </div>
</template>

<style scoped>
.props-panel {
  padding: 0.5rem;
}

.props-loading {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  justify-content: center;
  min-height: 200px;
  color: var(--musea-text-muted);
}

.loading-spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--musea-border);
  border-top-color: var(--musea-accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.props-error {
  padding: 1rem;
  color: var(--musea-error);
  background: rgba(248, 113, 113, 0.1);
  border: 1px solid rgba(248, 113, 113, 0.2);
  border-radius: var(--musea-radius-md);
  font-size: 0.8125rem;
}

.props-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1.25rem;
}

.props-title {
  font-size: 0.875rem;
  font-weight: 600;
}

.props-reset {
  background: var(--musea-bg-tertiary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  color: var(--musea-text-muted);
  font-size: 0.75rem;
  padding: 0.25rem 0.625rem;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.props-reset:hover {
  border-color: var(--musea-text-muted);
  color: var(--musea-text);
}

.props-grid {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.props-group-header {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--musea-text-muted);
  margin-top: 0.5rem;
  padding-bottom: 0.375rem;
  border-bottom: 1px solid var(--musea-border-subtle);
}

.props-json {
  margin-top: 1.5rem;
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  overflow: hidden;
}

.props-json-header {
  padding: 0.5rem 0.75rem;
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--musea-text-muted);
  background: var(--musea-bg-tertiary);
  border-bottom: 1px solid var(--musea-border);
}

.props-json-code {
  padding: 0.75rem;
  font-family: var(--musea-font-mono);
  font-size: 0.75rem;
  color: var(--musea-text-secondary);
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
}

.props-preview {
  margin-bottom: 1.25rem;
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  overflow: hidden;
}

.props-preview-header {
  padding: 0.5rem 0.75rem;
  background: var(--musea-bg-tertiary);
  border-bottom: 1px solid var(--musea-border);
}

.props-preview-label {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--musea-text-muted);
}

.props-preview-frame {
  aspect-ratio: 16 / 9;
  background: #fff;
}

.props-preview-frame iframe {
  width: 100%;
  height: 100%;
  border: none;
}

.props-slot-editor {
  margin-top: 1.25rem;
}

.props-slot-header {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--musea-text-muted);
  margin-bottom: 0.5rem;
}

.props-slot-textarea {
  width: 100%;
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  color: var(--musea-text);
  font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
  font-size: 0.75rem;
  padding: 0.5rem 0.75rem;
  resize: vertical;
  outline: none;
  transition: border-color var(--musea-transition);
}

.props-slot-textarea:focus {
  border-color: var(--musea-accent);
}

.props-slot-textarea::placeholder {
  color: var(--musea-text-muted);
}

.props-usage {
  margin-top: 1.25rem;
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  overflow: hidden;
}

.props-usage-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  background: var(--musea-bg-tertiary);
  border-bottom: 1px solid var(--musea-border);
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--musea-text-muted);
}

.props-copy-btn {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.125rem 0.375rem;
  border: 1px solid var(--musea-border);
  border-radius: 3px;
  background: var(--musea-bg-tertiary);
  color: var(--musea-text-muted);
  font-size: 0.625rem;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.props-copy-btn:hover {
  color: var(--musea-text);
  border-color: var(--musea-text-muted);
}

.props-usage-code {
  padding: 0.75rem;
  font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
  font-size: 0.75rem;
  color: var(--musea-text-secondary);
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
}

.props-empty {
  padding: 2rem;
  text-align: center;
  color: var(--musea-text-muted);
  font-size: 0.875rem;
}

.props-empty-hint {
  margin-top: 0.5rem;
  font-size: 0.8125rem;
}

.props-empty code {
  background: var(--musea-bg-tertiary);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
  font-family: var(--musea-font-mono);
}
</style>
