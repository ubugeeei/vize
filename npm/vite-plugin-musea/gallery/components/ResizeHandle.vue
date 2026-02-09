<script setup lang="ts">
defineProps<{
  direction: 'horizontal' | 'vertical';
  isResizing?: boolean;
}>();

const emit = defineEmits<{
  (e: 'mousedown', event: MouseEvent): void;
}>();
</script>

<template>
  <div
    :class="[
      'resize-handle',
      `resize-handle--${direction}`,
      { 'resize-handle--active': isResizing }
    ]"
    @mousedown="emit('mousedown', $event)"
  >
    <div class="resize-handle__indicator" />
  </div>
</template>

<style scoped>
.resize-handle {
  position: relative;
  flex-shrink: 0;
  background: transparent;
  transition: background-color 0.15s;
  z-index: 10;
}

.resize-handle--horizontal {
  width: 5px;
  cursor: col-resize;
}

.resize-handle--vertical {
  height: 5px;
  cursor: row-resize;
}

.resize-handle:hover,
.resize-handle--active {
  background: rgba(224, 112, 72, 0.3);
}

.resize-handle__indicator {
  position: absolute;
  background: var(--musea-border);
  transition: background-color 0.15s;
}

.resize-handle--horizontal .resize-handle__indicator {
  left: 50%;
  top: 0;
  bottom: 0;
  width: 1px;
  transform: translateX(-50%);
}

.resize-handle--vertical .resize-handle__indicator {
  top: 50%;
  left: 0;
  right: 0;
  height: 1px;
  transform: translateY(-50%);
}

.resize-handle:hover .resize-handle__indicator,
.resize-handle--active .resize-handle__indicator {
  background: var(--musea-accent);
}
</style>
