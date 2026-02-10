<script lang="ts">
export type { GraphAreaProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import type { GraphAreaProps } from './types'
import { injectGraphRootContext } from './types'
import { areaPath, monotoneAreaPath } from './utils'

const {
  color = 'currentColor',
  opacity = 0.3,
  curve = 'linear',
} = defineProps<GraphAreaProps>()

const context = injectGraphRootContext('GraphArea')

const pathD = computed(() => {
  const points: [number, number][] = context.data.map(d => [
    context.xScale.value(context.getX(d)),
    context.yScale.value(context.getY(d)),
  ])
  const baseline = context.padding.top + context.innerHeight.value
  return curve === 'monotone'
    ? monotoneAreaPath(points, baseline)
    : areaPath(points, baseline)
})
</script>

<template>
  <path
    :d="pathD"
    :fill="color"
    :opacity="opacity"
    stroke="none"
    role="presentation"
  />
</template>
