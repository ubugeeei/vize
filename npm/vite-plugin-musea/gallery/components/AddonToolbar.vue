<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useAddons } from '../composables/useAddons'
import ViewportSelector from './ViewportSelector.vue'
import BackgroundPicker from './BackgroundPicker.vue'

const {
  outlineEnabled,
  measureEnabled,
  multiViewportEnabled,
  gridDensity,
  toggleOutline,
  toggleMeasure,
  toggleMultiViewport,
  setGridDensity,
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
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          <line x1="3" y1="9" x2="21" y2="9" />
          <line x1="9" y1="21" x2="9" y2="9" />
        </svg>
        <span>Outline</span>
        <kbd class="toolbar-kbd">Alt+O</kbd>
      </button>

      <button
        class="toolbar-toggle"
        :class="{ active: measureEnabled }"
        title="Toggle Measure (Alt+M)"
        @click="toggleMeasure()"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
          <path d="M2 12h20M12 2v20M2 7h5M2 17h5M17 2v5M17 17v5M7 2v5M7 17v5M19 7h3M19 17h3" />
        </svg>
        <span>Measure</span>
        <kbd class="toolbar-kbd">Alt+M</kbd>
      </button>

      <button
        class="toolbar-toggle"
        :class="{ active: multiViewportEnabled }"
        title="Multi-Viewport Comparison"
        @click="toggleMultiViewport()"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
          <rect x="2" y="3" width="20" height="18" rx="2" />
          <line x1="9" y1="3" x2="9" y2="21" />
          <line x1="16" y1="3" x2="16" y2="21" />
        </svg>
        <span>Multi</span>
      </button>
    </div>

    <div class="toolbar-separator" />

    <div class="toolbar-group">
      <div class="density-selector">
        <button
          v-for="d in (['compact', 'comfortable', 'spacious'] as const)"
          :key="d"
          class="density-btn"
          :class="{ active: gridDensity === d }"
          :title="`Grid: ${d}`"
          @click="setGridDensity(d)"
        >
          <svg v-if="d === 'compact'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12">
            <rect x="3" y="3" width="7" height="7" /><rect x="14" y="3" width="7" height="7" />
            <rect x="3" y="14" width="7" height="7" /><rect x="14" y="14" width="7" height="7" />
          </svg>
          <svg v-else-if="d === 'comfortable'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12">
            <rect x="3" y="3" width="8" height="8" /><rect x="13" y="3" width="8" height="8" />
            <rect x="3" y="13" width="8" height="8" /><rect x="13" y="13" width="8" height="8" />
          </svg>
          <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12">
            <rect x="4" y="4" width="16" height="16" rx="2" />
          </svg>
        </button>
      </div>
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

.density-selector {
  display: flex;
  border: 1px solid var(--musea-border);
  border-radius: 2px;
  overflow: hidden;
}

.density-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 16px;
  border: none;
  background: var(--musea-bg-tertiary);
  color: var(--musea-text-muted);
  cursor: pointer;
  transition: all var(--musea-transition);
}

.density-btn:not(:last-child) {
  border-right: 1px solid var(--musea-border);
}

.density-btn:hover {
  color: var(--musea-text);
}

.density-btn.active {
  background: var(--musea-accent-subtle);
  color: var(--musea-accent);
}

.density-btn svg {
  width: 10px;
  height: 10px;
}

@media (min-width: 1024px) {
  .toolbar-kbd {
    display: inline-block;
  }
}
</style>
