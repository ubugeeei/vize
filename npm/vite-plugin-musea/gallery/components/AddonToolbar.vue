<script setup lang="ts">
import { useAddons } from '../composables/useAddons'
import ViewportSelector from './ViewportSelector.vue'
import BackgroundPicker from './BackgroundPicker.vue'

const { outlineEnabled, measureEnabled, toggleOutline, toggleMeasure } = useAddons()
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
        title="Toggle Outline"
        @click="toggleOutline()"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          <line x1="3" y1="9" x2="21" y2="9" />
          <line x1="9" y1="21" x2="9" y2="9" />
        </svg>
        <span>Outline</span>
      </button>

      <button
        class="toolbar-toggle"
        :class="{ active: measureEnabled }"
        title="Toggle Measure"
        @click="toggleMeasure()"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
          <path d="M2 12h20M12 2v20M2 7h5M2 17h5M17 2v5M17 17v5M7 2v5M7 17v5M19 7h3M19 17h3" />
        </svg>
        <span>Measure</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.addon-toolbar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  flex-wrap: wrap;
}

.toolbar-group {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.toolbar-separator {
  width: 1px;
  height: 20px;
  background: var(--musea-border);
  flex-shrink: 0;
}

.toolbar-toggle {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.25rem 0.5rem;
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  background: var(--musea-bg-tertiary);
  color: var(--musea-text-muted);
  font-size: 0.6875rem;
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
</style>
