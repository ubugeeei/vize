<script lang="ts">
export type { GraphRootProps, GraphRootContext } from './types'
export { injectGraphRootContext, provideGraphRootContext } from './types'
</script>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { GraphRootProps, GraphDataPoint } from './types'
import { provideGraphRootContext } from './types'
import { createLinearScale, extent } from './utils'

const {
  width: widthProp = '100%',
  height: heightProp = 300,
  padding: paddingProp,
  data,
  xAccessor = 'x',
  yAccessor = 'y',
} = defineProps<GraphRootProps>()

const svgRef = ref<SVGSVGElement>()

const padding = {
  top: paddingProp?.top ?? 20,
  right: paddingProp?.right ?? 20,
  bottom: paddingProp?.bottom ?? 30,
  left: paddingProp?.left ?? 40,
}

function getX(d: GraphDataPoint): number {
  if (typeof xAccessor === 'function') return xAccessor(d)
  return Number(d[xAccessor]) || 0
}

function getY(d: GraphDataPoint): number {
  if (typeof yAccessor === 'function') return yAccessor(d)
  return Number(d[yAccessor]) || 0
}

const resolvedWidth = computed(() => {
  if (typeof widthProp === 'number') return widthProp
  return 600 // fallback for percentage; actual sizing handled by SVG viewBox
})

const resolvedHeight = computed(() => {
  if (typeof heightProp === 'number') return heightProp
  return Number.parseInt(String(heightProp), 10) || 300
})

const innerWidth = computed(() => resolvedWidth.value - padding.left - padding.right)
const innerHeight = computed(() => resolvedHeight.value - padding.top - padding.bottom)

const xScale = computed(() => {
  const [min, max] = extent(data, getX)
  return createLinearScale([min, max], [padding.left, padding.left + innerWidth.value])
})

const yScale = computed(() => {
  const [min, max] = extent(data, getY)
  return createLinearScale([min, max], [padding.top + innerHeight.value, padding.top])
})

provideGraphRootContext({
  width: resolvedWidth,
  height: resolvedHeight,
  padding,
  innerWidth,
  innerHeight,
  data,
  xScale,
  yScale,
  getX,
  getY,
})
</script>

<template>
  <svg
    ref="svgRef"
    :width="widthProp"
    :height="resolvedHeight"
    :viewBox="`0 0 ${resolvedWidth} ${resolvedHeight}`"
    role="img"
    preserveAspectRatio="xMidYMid meet"
  >
    <slot
      :width="resolvedWidth"
      :height="resolvedHeight"
      :x-scale="xScale"
      :y-scale="yScale"
    />
  </svg>
</template>
