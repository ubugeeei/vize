<script setup lang="ts">
import { ref, watch, computed, defineAsyncComponent } from 'vue'

const MonacoEditor = defineAsyncComponent(() => import('./MonacoEditor.vue'))

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

</script>

<template>
  <div class="slot-editor">
    <div class="slot-header">
      <div class="slot-tabs">
        <button
          v-for="name in slotNames"
          :key="name"
          type="button"
          :class="['slot-tab', { 'slot-tab--active': activeSlot === name }]"
          @click="selectSlot(name)"
        >
          <span class="slot-tab-icon">#</span>
          {{ name }}
        </button>
      </div>
      <div class="slot-actions">
        <button type="button" class="slot-action" @click="clearSlot" title="Clear current slot">
          Clear
        </button>
        <button type="button" class="slot-action slot-action--danger" @click="clearAllSlots" title="Clear all slots">
          Clear All
        </button>
      </div>
    </div>

    <div class="slot-content">
      <MonacoEditor
        v-model="currentContent"
        language="html"
        height="150px"
      />
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
  flex: 1;
  overflow: hidden;
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
