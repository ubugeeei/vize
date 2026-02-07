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
  background: #f3f4f6;
  color: #374151;
}

.badge--success {
  background: #dcfce7;
  color: #166534;
}

.badge--warning {
  background: #fef3c7;
  color: #92400e;
}

.badge--error {
  background: #fee2e2;
  color: #991b1b;
}

.badge--info {
  background: #dbeafe;
  color: #1e40af;
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
