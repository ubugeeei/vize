<script lang="ts">
export type { GraphGridProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import type { GraphGridProps } from './types'
import { injectGraphRootContext } from './types'

const {
  horizontal = true,
  vertical = false,
  tickCount = 5,
} = defineProps<GraphGridProps>()

const context = injectGraphRootContext('GraphGrid')

const horizontalLines = computed(() => {
  if (!horizontal) return []
  return context.yScale.value.ticks(tickCount).map(value => ({
    value,
    y: context.yScale.value(value),
    x1: context.padding.left,
    x2: context.padding.left + context.innerWidth.value,
  }))
})

const verticalLines = computed(() => {
  if (!vertical) return []
  return context.xScale.value.ticks(tickCount).map(value => ({
    value,
    x: context.xScale.value(value),
    y1: context.padding.top,
    y2: context.padding.top + context.innerHeight.value,
  }))
})
</script>

<template>
  <g aria-hidden="true">
    <line
      v-for="(line, index) in horizontalLines"
      :key="`h-${index}`"
      :x1="line.x1"
      :x2="line.x2"
      :y1="line.y"
      :y2="line.y"
      stroke="currentColor"
      stroke-opacity="0.15"
      stroke-width="1"
    />
    <line
      v-for="(line, index) in verticalLines"
      :key="`v-${index}`"
      :x1="line.x"
      :x2="line.x"
      :y1="line.y1"
      :y2="line.y2"
      stroke="currentColor"
      stroke-opacity="0.15"
      stroke-width="1"
    />
  </g>
</template>
