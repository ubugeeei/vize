<script setup lang="ts">
defineProps<{
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info'
  size?: 'sm' | 'md' | 'lg'
}>()
</script>

<template>
  <span
    class="badge"
    :class="[`badge--${variant ?? 'default'}`, `badge--${size ?? 'md'}`]"
  >
    <slot />
  </span>
</template>

<style scoped>
.badge {
  display: inline-flex;
  align-items: center;
  font-weight: 600;
  border-radius: 9999px;
  white-space: nowrap;
}

.badge--sm {
  padding: 0.125rem 0.5rem;
  font-size: 0.625rem;
}

.badge--md {
  padding: 0.25rem 0.625rem;
  font-size: 0.75rem;
}

.badge--lg {
  padding: 0.375rem 0.75rem;
  font-size: 0.875rem;
}

.badge--default {
  background: #d4d0c4;
  color: #3a3a3a;
}

.badge--success {
  background: rgba(45, 106, 53, 0.12);
  color: #2d6a35;
}

.badge--warning {
  background: rgba(139, 112, 64, 0.12);
  color: #8b7040;
}

.badge--error {
  background: rgba(160, 64, 64, 0.1);
  color: #a04040;
}

.badge--info {
  background: rgba(74, 107, 138, 0.12);
  color: #4a6b8a;
}
</style>

<art title="Badge" category="Components" status="ready" tags="badge,tag,label">
  <variant name="Default" default>
    <Self>Default</Self>
  </variant>
  <variant name="Variants">
    <div style="display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: center">
      <Self>Default</Self>
      <Self variant="success">Success</Self>
      <Self variant="warning">Warning</Self>
      <Self variant="error">Error</Self>
      <Self variant="info">Info</Self>
    </div>
  </variant>
  <variant name="Sizes">
    <div style="display: flex; gap: 0.5rem; align-items: center">
      <Self size="sm" variant="info">Small</Self>
      <Self size="md" variant="info">Medium</Self>
      <Self size="lg" variant="info">Large</Self>
    </div>
  </variant>
</art>
