<script setup lang="ts">
defineProps<{
  variant?: 'default' | 'primary' | 'secondary'
  disabled?: boolean
}>()
</script>

<template>
  <button
    class="btn"
    :class="[`btn--${variant ?? 'default'}`, { 'btn--disabled': disabled }]"
    :disabled="disabled"
  >
    <slot />
  </button>
</template>

<style scoped>
.btn {
  padding: 0.5rem 1rem;
  border: 1px solid #ccc;
  border-radius: 4px;
  cursor: pointer;
  font-size: 1rem;
  transition: all 0.2s;
}

.btn--default {
  background: #fff;
  color: #333;
}

.btn--default:hover:not(:disabled) {
  background: #f5f5f5;
}

.btn--primary {
  background: #42b883;
  border-color: #42b883;
  color: #fff;
}

.btn--primary:hover:not(:disabled) {
  background: #33a06f;
}

.btn--secondary {
  background: #35495e;
  border-color: #35495e;
  color: #fff;
}

.btn--secondary:hover:not(:disabled) {
  background: #2c3e50;
}

.btn--disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>

<art title="Button (Inline)" category="Inline Components" status="ready">
  <variant name="Default" default>
    <Self>Default Button</Self>
  </variant>
  <variant name="Primary">
    <Self variant="primary">Primary Button</Self>
  </variant>
  <variant name="Secondary">
    <Self variant="secondary">Secondary Button</Self>
  </variant>
  <variant name="Disabled">
    <Self disabled>Disabled Button</Self>
  </variant>
  <variant name="All Variants" skip-vrt>
    <div style="display: flex; gap: 1rem; flex-wrap: wrap">
      <Self>Default</Self>
      <Self variant="primary">Primary</Self>
      <Self variant="secondary">Secondary</Self>
      <Self disabled>Disabled</Self>
    </div>
  </variant>
</art>
