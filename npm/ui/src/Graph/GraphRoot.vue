<script lang="ts">
export type { GraphRootProps, GraphRootContext } from './types'
export { injectGraphRootContext, provideGraphRootContext } from './types'
</script>

<script setup lang="ts">
import { computed, ref, useAttrs } from 'vue'
import type { GraphRootProps, GraphDataPoint } from './types'
import { provideGraphRootContext } from './types'
import { createLinearScale, extent } from './utils'

// Rust compiler cannot resolve imported types (GraphRootProps is from ./types),
// so no runtime `props` option is generated in defineComponent.
// All passed props end up in $attrs. We use useAttrs() to access them.
defineProps<GraphRootProps>()
const attrs = useAttrs()

// Helper: attrs use kebab-case when props are not declared
function attr(name: string): unknown {
  if (name in attrs) return attrs[name]
  const kebab = name.replace(/[A-Z]/g, (m: string) => '-' + m.toLowerCase())
  return attrs[kebab]
}

const svgRef = ref<SVGSVGElement>()

const resolvedPadding = computed(() => {
  const p = attr('padding') as { top?: number; right?: number; bottom?: number; left?: number } | undefined
  return {
    top: p?.top ?? 20,
    right: p?.right ?? 20,
    bottom: p?.bottom ?? 30,
    left: p?.left ?? 40,
  }
})

function getX(d: GraphDataPoint): number {
  const acc = attr('xAccessor') ?? 'x'
  if (typeof acc === 'function') return acc(d)
  const key = String(acc)
  return Number(d[key]) || 0
}

function getY(d: GraphDataPoint): number {
  const acc = attr('yAccessor') ?? 'y'
  if (typeof acc === 'function') return acc(d)
  const key = String(acc)
  return Number(d[key]) || 0
}

const svgWidth = computed(() => attr('width') ?? '100%')

const resolvedWidth = computed(() => {
  const w = attr('width') ?? '100%'
  if (typeof w === 'number') return w
  return 600
})

const resolvedHeight = computed(() => {
  const h = attr('height') ?? 300
  if (typeof h === 'number') return h
  return Number.parseInt(String(h), 10) || 300
})

const innerWidth = computed(() => resolvedWidth.value - resolvedPadding.value.left - resolvedPadding.value.right)
const innerHeight = computed(() => resolvedHeight.value - resolvedPadding.value.top - resolvedPadding.value.bottom)

const xScale = computed(() => {
  const d = (attr('data') ?? []) as GraphDataPoint[]
  if (d.length === 0) return createLinearScale([0, 1], [resolvedPadding.value.left, resolvedPadding.value.left + innerWidth.value])
  const [min, max] = extent(d, getX)
  return createLinearScale([min, max], [resolvedPadding.value.left, resolvedPadding.value.left + innerWidth.value])
})

const yScale = computed(() => {
  const d = (attr('data') ?? []) as GraphDataPoint[]
  if (d.length === 0) return createLinearScale([0, 1], [resolvedPadding.value.top + innerHeight.value, resolvedPadding.value.top])
  const [min, max] = extent(d, getY)
  return createLinearScale([min, max], [resolvedPadding.value.top + innerHeight.value, resolvedPadding.value.top])
})

provideGraphRootContext({
  width: resolvedWidth,
  height: resolvedHeight,
  padding: resolvedPadding.value,
  innerWidth,
  innerHeight,
  data: (attr('data') ?? []) as GraphDataPoint[],
  xScale,
  yScale,
  getX,
  getY,
})
</script>

<template>
  <svg
    ref="svgRef"
    :width="svgWidth"
    :height="resolvedHeight"
    :viewBox="'0 0 ' + resolvedWidth + ' ' + resolvedHeight"
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
