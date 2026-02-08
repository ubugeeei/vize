<script setup lang="ts">
defineProps<{
  path: string
  size?: number | string
}>()
</script>

<template>
  <svg
    class="mdi-icon"
    :style="{ width: typeof size === 'number' ? `${size}px` : size, height: typeof size === 'number' ? `${size}px` : size }"
    viewBox="0 0 24 24"
  >
    <path :d="path" fill="currentColor" />
  </svg>
</template>

<style scoped>
.mdi-icon {
  width: 24px;
  height: 24px;
  flex-shrink: 0;
}
</style>
