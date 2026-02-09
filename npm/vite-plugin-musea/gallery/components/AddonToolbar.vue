<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { mdiSquareOutline, mdiRulerSquare } from '@mdi/js'
import { useAddons } from '../composables/useAddons'
import ViewportSelector from './ViewportSelector.vue'
import BackgroundPicker from './BackgroundPicker.vue'
import MdiIcon from './MdiIcon.vue'

const {
  outlineEnabled,
  measureEnabled,
  toggleOutline,
  toggleMeasure,
} = useAddons()

function onKeydown(e: KeyboardEvent) {
  // Alt+O: toggle outline
  if (e.altKey && e.key === 'o') {
    e.preventDefault()
    toggleOutline()
  }
  // Alt+M: toggle measure
  if (e.altKey && e.key === 'm') {
    e.preventDefault()
    toggleMeasure()
  }
}

onMounted(() => document.addEventListener('keydown', onKeydown))
onUnmounted(() => document.removeEventListener('keydown', onKeydown))
</script>

<template>
  <div class="addon-toolbar">
    <div class="toolbar-group">
      <ViewportSelector />
    </div>

    <div class="toolbar-separator" />

    <div class="toolbar-group">
      <BackgroundPicker />
    </div>

    <div class="toolbar-separator" />

    <div class="toolbar-group">
      <button
        class="toolbar-toggle"
        :class="{ active: outlineEnabled }"
        title="Toggle Outline (Alt+O)"
        @click="toggleOutline()"
      >
        <MdiIcon :path="mdiSquareOutline" :size="14" />
        <span>Outline</span>
        <kbd class="toolbar-kbd">Alt+O</kbd>
      </button>

      <button
        class="toolbar-toggle"
        :class="{ active: measureEnabled }"
        title="Toggle Measure (Alt+M)"
        @click="toggleMeasure()"
      >
        <MdiIcon :path="mdiRulerSquare" :size="14" />
        <span>Measure</span>
        <kbd class="toolbar-kbd">Alt+M</kbd>
      </button>
    </div>
  </div>
</template>

<style scoped>
.addon-toolbar {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.125rem 0.25rem;
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: 2px;
  flex-wrap: wrap;
}

.toolbar-group {
  display: flex;
  align-items: center;
  gap: 0.125rem;
}

.toolbar-separator {
  width: 1px;
  height: 14px;
  background: var(--musea-border);
  flex-shrink: 0;
}

.toolbar-toggle {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.125rem 0.25rem;
  border: 1px solid var(--musea-border);
  border-radius: 2px;
  background: var(--musea-bg-tertiary);
  color: var(--musea-text-muted);
  font-size: 0.5625rem;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.toolbar-toggle:hover {
  border-color: var(--musea-text-muted);
  color: var(--musea-text);
}

.toolbar-toggle.active {
  border-color: var(--musea-accent);
  color: var(--musea-accent);
  background: var(--musea-accent-subtle);
}

.toolbar-toggle svg {
  width: 10px;
  height: 10px;
}

.toolbar-kbd {
  display: none;
  padding: 0 0.125rem;
  border: 1px solid var(--musea-border);
  border-radius: 2px;
  background: var(--musea-bg-primary);
  font-family: var(--musea-font-mono, monospace);
  font-size: 0.5rem;
  color: var(--musea-text-muted);
  line-height: 1.2;
}

@media (min-width: 1024px) {
  .toolbar-kbd {
    display: inline-block;
  }
}
</style>
