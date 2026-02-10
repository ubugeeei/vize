<script lang="ts">
export type { GraphDotProps } from './types'
</script>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { GraphDotProps } from './types'
import { injectGraphRootContext } from './types'

const {
  radius = 4,
  color = 'currentColor',
  activeRadius = 6,
} = defineProps<GraphDotProps>()

const context = injectGraphRootContext('GraphDot')

const activeIndex = ref(-1)

const dots = computed(() => {
  return context.data.map((d, i) => ({
    cx: context.xScale.value(context.getX(d)),
    cy: context.yScale.value(context.getY(d)),
    r: activeIndex.value === i ? activeRadius : radius,
    index: i,
  }))
})

function handleMouseEnter(index: number) {
  activeIndex.value = index
}

function handleMouseLeave() {
  activeIndex.value = -1
}
</script>

<template>
  <g role="presentation">
    <circle
      v-for="dot in dots"
      :key="dot.index"
      :cx="dot.cx"
      :cy="dot.cy"
      :r="dot.r"
      :fill="color"
      @mouseenter="handleMouseEnter(dot.index)"
      @mouseleave="handleMouseLeave"
    />
  </g>
</template>
