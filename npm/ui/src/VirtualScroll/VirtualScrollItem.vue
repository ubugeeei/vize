<script lang="ts">
export type { VirtualScrollContentProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { VirtualScrollContentProps, VirtualItem } from './types'

const {
  as = 'div',
  asChild = false,
  virtualItem,
  horizontal = false,
} = defineProps<VirtualScrollContentProps & {
  virtualItem: VirtualItem
  horizontal?: boolean
}>()

const itemStyle = computed(() => ({
  position: 'absolute',
  [horizontal ? 'left' : 'top']: `${virtualItem.start}px`,
  [horizontal ? 'height' : 'width']: '100%',
  [horizontal ? 'width' : 'height']: `${virtualItem.size}px`,
}))
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="listitem"
    :aria-rowindex="virtualItem.index + 1"
    :style="itemStyle"
  >
    <slot />
  </Primitive>
</template>
