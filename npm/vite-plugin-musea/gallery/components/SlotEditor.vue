<script setup lang="ts">
import { ref, watch, computed } from 'vue'

const props = defineProps<{
  slots: Record<string, string>
  availableSlots?: string[]
}>()

const emit = defineEmits<{
  (e: 'update', slots: Record<string, string>): void
}>()

const activeSlot = ref('default')
const localSlots = ref<Record<string, string>>({})

// Initialize local slots from props
watch(() => props.slots, (newSlots) => {
  localSlots.value = { ...newSlots }
}, { immediate: true, deep: true })

const slotNames = computed(() => {
  const names = new Set(['default'])
  if (props.availableSlots) {
    for (const name of props.availableSlots) {
      names.add(name)
    }
  }
  for (const name of Object.keys(localSlots.value)) {
    names.add(name)
  }
  return Array.from(names)
})

const currentContent = computed({
  get: () => localSlots.value[activeSlot.value] || '',
  set: (value: string) => {
    localSlots.value[activeSlot.value] = value
    emit('update', { ...localSlots.value })
  }
})

const selectSlot = (name: string) => {
  activeSlot.value = name
}

const clearSlot = () => {
  localSlots.value[activeSlot.value] = ''
  emit('update', { ...localSlots.value })
}

const clearAllSlots = () => {
  localSlots.value = {}
  emit('update', {})
}

// Simple syntax highlighting for HTML
const highlightedContent = computed(() => {
  const content = currentContent.value
  if (!content) return ''

  return content
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    // Highlight tags
    .replace(/(&lt;\/?)([\w-]+)([^&]*?)(&gt;)/g,
      '<span class="tag-bracket">$1</span><span class="tag-name">$2</span>$3<span class="tag-bracket">$4</span>')
    // Highlight attributes
    .replace(/([\w-]+)(=)(&quot;[^&]*&quot;|&#39;[^&]*&#39;)/g,
      '<span class="attr-name">$1</span><span class="attr-eq">$2</span><span class="attr-value">$3</span>')
})
</script>

<template>
  <div class="slot-editor">
    <div class="slot-header">
      <div class="slot-tabs">
        <button
          v-for="name in slotNames"
          :key="name"
          :class="['slot-tab', { 'slot-tab--active': activeSlot === name }]"
          @click="selectSlot(name)"
        >
          <span class="slot-tab-icon">#</span>
          {{ name }}
        </button>
      </div>
      <div class="slot-actions">
        <button class="slot-action" @click="clearSlot" title="Clear current slot">
          Clear
        </button>
        <button class="slot-action slot-action--danger" @click="clearAllSlots" title="Clear all slots">
          Clear All
        </button>
      </div>
    </div>

    <div class="slot-content">
      <div class="editor-wrapper">
        <textarea
          v-model="currentContent"
          class="slot-textarea"
          :placeholder="`Enter HTML content for #${activeSlot} slot...`"
          spellcheck="false"
        />
        <!-- Syntax highlighting overlay (optional, simple version) -->
        <div v-if="false" class="syntax-overlay" v-html="highlightedContent" />
      </div>

      <div class="slot-preview">
        <div class="preview-header">Preview</div>
        <div class="preview-content" v-html="currentContent || '<span class=\'empty\'>Empty slot</span>'" />
      </div>
    </div>

    <div class="slot-footer">
      <div class="slot-hint">
        <code>&lt;slot&gt;</code> = default, <code>&lt;slot name="foo"&gt;</code> = #foo
      </div>
    </div>
  </div>
</template>

<style scoped>
.slot-editor {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--musea-bg-secondary);
  border-top: 1px solid var(--musea-border);
}

.slot-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem;
  background: var(--musea-bg-tertiary);
  border-bottom: 1px solid var(--musea-border);
}

.slot-tabs {
  display: flex;
  gap: 0.25rem;
}

.slot-tab {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.375rem 0.625rem;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 4px;
  font-size: 0.75rem;
  color: var(--musea-text-muted);
  cursor: pointer;
  transition: all 0.15s;
}

.slot-tab:hover {
  background: var(--musea-bg-secondary);
  color: var(--musea-text-secondary);
}

.slot-tab--active {
  background: var(--musea-bg-secondary);
  border-color: var(--musea-accent);
  color: var(--musea-text);
}

.slot-tab-icon {
  font-family: var(--musea-font-mono);
  color: var(--musea-accent);
}

.slot-actions {
  display: flex;
  gap: 0.25rem;
}

.slot-action {
  padding: 0.25rem 0.5rem;
  background: transparent;
  border: 1px solid var(--musea-border);
  border-radius: 3px;
  font-size: 0.6875rem;
  color: var(--musea-text-muted);
  cursor: pointer;
  transition: all 0.15s;
}

.slot-action:hover {
  background: var(--musea-bg-secondary);
  color: var(--musea-text);
}

.slot-action--danger:hover {
  border-color: #f87171;
  color: #f87171;
}

.slot-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.editor-wrapper {
  flex: 1;
  position: relative;
  overflow: hidden;
}

.slot-textarea {
  width: 100%;
  height: 100%;
  padding: 0.75rem;
  background: var(--musea-bg-primary);
  border: none;
  font-family: var(--musea-font-mono);
  font-size: 0.8125rem;
  line-height: 1.5;
  color: var(--musea-text);
  resize: none;
  outline: none;
}

.slot-textarea::placeholder {
  color: var(--musea-text-muted);
}

.syntax-overlay {
  position: absolute;
  inset: 0;
  padding: 0.75rem;
  font-family: var(--musea-font-mono);
  font-size: 0.8125rem;
  line-height: 1.5;
  pointer-events: none;
  white-space: pre-wrap;
  word-break: break-word;
}

.syntax-overlay :deep(.tag-bracket) {
  color: var(--musea-text-muted);
}

.syntax-overlay :deep(.tag-name) {
  color: #60a5fa;
}

.syntax-overlay :deep(.attr-name) {
  color: #fbbf24;
}

.syntax-overlay :deep(.attr-eq) {
  color: var(--musea-text-muted);
}

.syntax-overlay :deep(.attr-value) {
  color: #4ade80;
}

.slot-preview {
  width: 200px;
  border-left: 1px solid var(--musea-border);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

.preview-header {
  padding: 0.5rem 0.75rem;
  background: var(--musea-bg-tertiary);
  border-bottom: 1px solid var(--musea-border);
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--musea-text-muted);
}

.preview-content {
  flex: 1;
  padding: 0.75rem;
  overflow: auto;
  background: #ffffff;
  font-size: 0.875rem;
}

.preview-content :deep(.empty) {
  color: #9ca3af;
  font-style: italic;
}

.slot-footer {
  padding: 0.375rem 0.75rem;
  background: var(--musea-bg-tertiary);
  border-top: 1px solid var(--musea-border);
}

.slot-hint {
  font-size: 0.6875rem;
  color: var(--musea-text-muted);
}

.slot-hint code {
  padding: 0.0625rem 0.25rem;
  background: var(--musea-bg-primary);
  border-radius: 2px;
  font-family: var(--musea-font-mono);
}
</style>
