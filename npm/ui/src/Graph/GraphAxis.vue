<script lang="ts">
export type { GraphAxisProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import type { GraphAxisProps } from './types'
import { injectGraphRootContext } from './types'

const {
  position,
  tickCount = 5,
  formatTick,
} = defineProps<GraphAxisProps>()

const context = injectGraphRootContext('GraphAxis')

const isHorizontal = computed(() => position === 'top' || position === 'bottom')

const scale = computed(() => isHorizontal.value ? context.xScale.value : context.yScale.value)

const ticks = computed(() => scale.value.ticks(tickCount))

const axisTransform = computed(() => {
  switch (position) {
    case 'bottom':
      return `translate(0, ${context.padding.top + context.innerHeight.value})`
    case 'top':
      return `translate(0, ${context.padding.top})`
    case 'left':
      return `translate(${context.padding.left}, 0)`
    case 'right':
      return `translate(${context.padding.left + context.innerWidth.value}, 0)`
    default:
      return ''
  }
})

const tickSize = 6

function formatValue(value: number): string {
  if (formatTick) return formatTick(value)
  return String(value)
}

function tickTransform(value: number): string {
  const pos = scale.value(value)
  return isHorizontal.value
    ? `translate(${pos}, 0)`
    : `translate(0, ${pos})`
}

const textAnchor = computed(() => {
  if (isHorizontal.value) return 'middle'
  return position === 'left' ? 'end' : 'start'
})

const textDx = computed(() => {
  if (isHorizontal.value) return 0
  return position === 'left' ? -tickSize - 4 : tickSize + 4
})

const textDy = computed(() => {
  if (!isHorizontal.value) return '0.32em'
  return position === 'bottom' ? tickSize + 14 : -(tickSize + 4)
})

const lineX2 = computed(() => {
  if (isHorizontal.value) return 0
  return position === 'left' ? -tickSize : tickSize
})

const lineY2 = computed(() => {
  if (!isHorizontal.value) return 0
  return position === 'bottom' ? tickSize : -tickSize
})

const axisLineD = computed(() => {
  if (isHorizontal.value) {
    const [r0, r1] = scale.value.range
    return `M${r0},0L${r1},0`
  }
  const [r0, r1] = scale.value.range
  return `M0,${r0}L0,${r1}`
})
</script>

<template>
  <g :transform="axisTransform" aria-hidden="true">
    <path
      :d="axisLineD"
      fill="none"
      stroke="currentColor"
      stroke-width="1"
    />
    <g
      v-for="(tick, index) in ticks"
      :key="index"
      :transform="tickTransform(tick)"
    >
      <line
        :x2="lineX2"
        :y2="lineY2"
        stroke="currentColor"
        stroke-width="1"
      />
      <text
        :x="textDx"
        :dy="textDy"
        :text-anchor="textAnchor"
        fill="currentColor"
        font-size="12"
      >
        {{ formatValue(tick) }}
      </text>
    </g>
  </g>
</template>
