<script lang="ts">
export type { GraphLineProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import type { GraphLineProps } from './types'
import { injectGraphRootContext } from './types'
import { linePath, monotonePath } from './utils'

const {
  color = 'currentColor',
  strokeWidth = 2,
  curve = 'linear',
} = defineProps<GraphLineProps>()

const context = injectGraphRootContext('GraphLine')

const pathD = computed(() => {
  const points: [number, number][] = context.data.map(d => [
    context.xScale.value(context.getX(d)),
    context.yScale.value(context.getY(d)),
  ])
  return curve === 'monotone' ? monotonePath(points) : linePath(points)
})
</script>

<template>
  <path
    :d="pathD"
    :stroke="color"
    :stroke-width="strokeWidth"
    fill="none"
    stroke-linecap="round"
    stroke-linejoin="round"
    role="presentation"
  />
</template>
