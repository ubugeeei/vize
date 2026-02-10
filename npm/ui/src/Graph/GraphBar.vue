<script lang="ts">
export type { GraphBarProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import type { GraphBarProps } from './types'
import { injectGraphRootContext } from './types'

const {
  color = 'currentColor',
  radius = 0,
  gap = 2,
} = defineProps<GraphBarProps>()

const context = injectGraphRootContext('GraphBar')

const bars = computed(() => {
  const n = context.data.length
  if (n === 0) return []

  const barWidth = Math.max(1, (context.innerWidth.value / n) - gap)
  const baseline = context.padding.top + context.innerHeight.value

  return context.data.map((d) => {
    const x = context.xScale.value(context.getX(d))
    const y = context.yScale.value(context.getY(d))
    const h = baseline - y

    return {
      x: x - barWidth / 2,
      y,
      width: barWidth,
      height: Math.max(0, h),
      rx: Math.min(radius, barWidth / 2),
    }
  })
})
</script>

<template>
  <g role="presentation">
    <rect
      v-for="(bar, index) in bars"
      :key="index"
      :x="bar.x"
      :y="bar.y"
      :width="bar.width"
      :height="bar.height"
      :rx="bar.rx"
      :fill="color"
    />
  </g>
</template>
