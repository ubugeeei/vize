<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useAddons, VIEWPORT_PRESETS } from '../composables/useAddons'

const { viewport, viewportRotated, setViewport, rotateViewport } = useAddons()
const showDropdown = ref(false)
const selectorRef = ref<HTMLElement | null>(null)

function selectPreset(preset: typeof VIEWPORT_PRESETS[number]) {
  setViewport(preset)
  showDropdown.value = false
}

function onClickOutside(e: MouseEvent) {
  if (selectorRef.value && !selectorRef.value.contains(e.target as Node)) {
    showDropdown.value = false
  }
}

onMounted(() => document.addEventListener('click', onClickOutside))
onUnmounted(() => document.removeEventListener('click', onClickOutside))
</script>

<template>
  <div ref="selectorRef" class="viewport-selector">
    <button
      type="button"
      class="viewport-btn"
      :class="{ active: viewport.width !== '100%' }"
      @click="showDropdown = !showDropdown"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
        <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
        <line x1="8" y1="21" x2="16" y2="21" />
        <line x1="12" y1="17" x2="12" y2="21" />
      </svg>
      <span class="viewport-label">{{ viewport.name }}</span>
    </button>

    <button
      v-if="viewport.width !== '100%'"
      type="button"
      class="rotate-btn"
      title="Rotate"
      :class="{ active: viewportRotated }"
      @click="rotateViewport()"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
        <polyline points="1 4 1 10 7 10" />
        <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
      </svg>
    </button>

    <div v-if="showDropdown" class="viewport-dropdown">
      <button
        v-for="preset in VIEWPORT_PRESETS"
        :key="preset.name"
        type="button"
        class="viewport-option"
        :class="{ active: viewport.name === preset.name }"
        @click="selectPreset(preset)"
      >
        <span class="viewport-option-name">{{ preset.name }}</span>
        <span v-if="preset.width !== '100%'" class="viewport-option-size">
          {{ preset.width }} x {{ preset.height }}
        </span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.viewport-selector {
  position: relative;
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.viewport-btn {
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

.viewport-btn:hover {
  border-color: var(--musea-text-muted);
  color: var(--musea-text);
}

.viewport-btn.active {
  border-color: var(--musea-accent);
  color: var(--musea-accent);
  background: var(--musea-accent-subtle);
}

.viewport-label {
  white-space: nowrap;
}

.rotate-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  background: var(--musea-bg-tertiary);
  color: var(--musea-text-muted);
  cursor: pointer;
  transition: all var(--musea-transition);
}

.rotate-btn:hover {
  border-color: var(--musea-text-muted);
  color: var(--musea-text);
}

.rotate-btn.active {
  border-color: var(--musea-accent);
  color: var(--musea-accent);
}

.viewport-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  margin-top: 0.25rem;
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  box-shadow: var(--musea-shadow);
  min-width: 200px;
  z-index: 50;
  overflow: hidden;
}

.viewport-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  width: 100%;
  padding: 0.5rem 0.75rem;
  border: none;
  background: none;
  color: var(--musea-text-secondary);
  font-size: 0.75rem;
  cursor: pointer;
  transition: background var(--musea-transition);
  text-align: left;
}

.viewport-option:hover {
  background: var(--musea-bg-tertiary);
}

.viewport-option.active {
  color: var(--musea-accent);
  background: var(--musea-accent-subtle);
}

.viewport-option-size {
  color: var(--musea-text-muted);
  font-size: 0.6875rem;
  font-family: var(--musea-font-mono, monospace);
}
</style>
