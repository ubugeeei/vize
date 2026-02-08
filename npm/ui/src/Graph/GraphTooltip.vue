<script lang="ts">
export type { GraphTooltipProps } from './types'
</script>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import type { GraphTooltipProps, GraphDataPoint } from './types'
import { injectGraphRootContext } from './types'

const { snapToData = true } = defineProps<GraphTooltipProps>()

const context = injectGraphRootContext('GraphTooltip')

const mouseX = ref(0)
const mouseY = ref(0)
const isActive = ref(false)

const nearestIndex = computed(() => {
  if (!isActive.value || context.data.length === 0) return -1

  let minDist = Infinity
  let idx = 0

  for (let i = 0; i < context.data.length; i++) {
    const d = context.data[i]
    const px = context.xScale.value(context.getX(d))
    const py = context.yScale.value(context.getY(d))
    const dx = px - mouseX.value
    const dy = py - mouseY.value
    const dist = snapToData ? Math.abs(dx) : Math.sqrt(dx * dx + dy * dy)
    if (dist < minDist) {
      minDist = dist
      idx = i
    }
  }

  return idx
})

const nearestDataPoint = computed<GraphDataPoint | undefined>(() => {
  if (nearestIndex.value < 0) return undefined
  return context.data[nearestIndex.value]
})

const tooltipX = computed(() => {
  if (!snapToData || !nearestDataPoint.value) return mouseX.value
  return context.xScale.value(context.getX(nearestDataPoint.value))
})

const tooltipY = computed(() => {
  if (!snapToData || !nearestDataPoint.value) return mouseY.value
  return context.yScale.value(context.getY(nearestDataPoint.value))
})

let svgElement: SVGSVGElement | null = null

function getOwnerSvg(el: Element | null): SVGSVGElement | null {
  while (el) {
    if (el.tagName === 'svg') return el as SVGSVGElement
    el = el.parentElement
  }
  return null
}

function handleMouseMove(event: MouseEvent) {
  if (!svgElement) return
  const rect = svgElement.getBoundingClientRect()
  const scaleX = context.width.value / rect.width
  const scaleY = context.height.value / rect.height
  mouseX.value = (event.clientX - rect.left) * scaleX
  mouseY.value = (event.clientY - rect.top) * scaleY
}

function handleMouseEnter() {
  isActive.value = true
}

function handleMouseLeave() {
  isActive.value = false
}

const groupRef = ref<SVGGElement>()

onMounted(() => {
  const el = groupRef.value
  if (el) {
    svgElement = getOwnerSvg(el)
    if (svgElement) {
      svgElement.addEventListener('mousemove', handleMouseMove)
      svgElement.addEventListener('mouseenter', handleMouseEnter)
      svgElement.addEventListener('mouseleave', handleMouseLeave)
    }
  }
})

onBeforeUnmount(() => {
  if (svgElement) {
    svgElement.removeEventListener('mousemove', handleMouseMove)
    svgElement.removeEventListener('mouseenter', handleMouseEnter)
    svgElement.removeEventListener('mouseleave', handleMouseLeave)
  }
})
</script>

<template>
  <g ref="groupRef">
    <slot
      :x="tooltipX"
      :y="tooltipY"
      :data-point="nearestDataPoint"
      :index="nearestIndex"
      :is-active="isActive"
    />
  </g>
</template>
