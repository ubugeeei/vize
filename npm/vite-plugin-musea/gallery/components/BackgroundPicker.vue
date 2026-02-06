<script setup lang="ts">
import { ref } from 'vue'
import { useAddons, BG_PRESETS } from '../composables/useAddons'

const { background, setBackground, setCustomBackground } = useAddons()
const customColor = ref('#ffffff')

function onCustomColorInput(event: Event) {
  const value = (event.target as HTMLInputElement).value
  customColor.value = value
  setCustomBackground(value)
}
</script>

<template>
  <div class="bg-picker">
    <div class="bg-presets">
      <button
        v-for="preset in BG_PRESETS"
        :key="preset.name"
        class="bg-preset-btn"
        :class="{ active: background?.name === preset.name }"
        :title="preset.name"
        @click="setBackground(background?.name === preset.name ? null : preset)"
      >
        <span
          class="bg-preset-swatch"
          :class="{ 'checkerboard': preset.pattern === 'checkerboard' }"
          :style="preset.color !== 'transparent' ? { background: preset.color } : {}"
        />
        <span class="bg-preset-label">{{ preset.name }}</span>
      </button>
    </div>
    <div class="bg-custom">
      <input
        type="color"
        :value="customColor"
        class="bg-color-input"
        title="Custom color"
        @input="onCustomColorInput"
      >
    </div>
  </div>
</template>

<style scoped>
.bg-picker {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.bg-presets {
  display: flex;
  gap: 0.25rem;
}

.bg-preset-btn {
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

.bg-preset-btn:hover {
  border-color: var(--musea-text-muted);
  color: var(--musea-text);
}

.bg-preset-btn.active {
  border-color: var(--musea-accent);
  color: var(--musea-accent);
  background: var(--musea-accent-subtle);
}

.bg-preset-swatch {
  width: 14px;
  height: 14px;
  border-radius: 3px;
  border: 1px solid var(--musea-border);
  flex-shrink: 0;
}

.bg-preset-swatch.checkerboard {
  background-image:
    linear-gradient(45deg, #ccc 25%, transparent 25%),
    linear-gradient(-45deg, #ccc 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, #ccc 75%),
    linear-gradient(-45deg, transparent 75%, #ccc 75%);
  background-size: 8px 8px;
  background-position: 0 0, 0 4px, 4px -4px, -4px 0;
}

.bg-preset-label {
  white-space: nowrap;
}

.bg-color-input {
  width: 28px;
  height: 28px;
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  background: none;
  cursor: pointer;
  padding: 2px;
}
</style>
