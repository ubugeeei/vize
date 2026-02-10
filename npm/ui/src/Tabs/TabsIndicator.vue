<script lang="ts">
export type { TabsIndicatorProps } from './types'
</script>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { Primitive } from '../Primitive'
import { injectTabsRootContext } from './types'
import type { TabsIndicatorProps } from './types'

const { as = 'div', asChild = false } = defineProps<TabsIndicatorProps>()

const rootContext = injectTabsRootContext('TabsIndicator')

const indicatorLeft = ref(0)
const indicatorTop = ref(0)
const indicatorWidth = ref(0)
const indicatorHeight = ref(0)

let resizeObserver: ResizeObserver | undefined

function getActiveTrigger(): HTMLElement | null {
  if (!rootContext.parentRef.value) return null
  return rootContext.parentRef.value.querySelector<HTMLElement>(
    '[data-vize-collection-item][data-state="active"]',
  )
}

function updatePosition() {
  const activeTrigger = getActiveTrigger()
  if (!activeTrigger || !rootContext.parentRef.value) {
    indicatorWidth.value = 0
    indicatorHeight.value = 0
    return
  }

  const parentRect = rootContext.parentRef.value.getBoundingClientRect()
  const triggerRect = activeTrigger.getBoundingClientRect()

  indicatorLeft.value = triggerRect.left - parentRect.left
  indicatorTop.value = triggerRect.top - parentRect.top
  indicatorWidth.value = triggerRect.width
  indicatorHeight.value = triggerRect.height
}

function setupObserver() {
  cleanupObserver()
  const parent = rootContext.parentRef.value
  if (!parent) return

  resizeObserver = new ResizeObserver(() => {
    updatePosition()
  })

  const triggers = parent.querySelectorAll<HTMLElement>('[data-vize-collection-item]')
  triggers.forEach(trigger => resizeObserver!.observe(trigger))
  resizeObserver.observe(parent)
}

function cleanupObserver() {
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = undefined
  }
}

watch(() => rootContext.modelValue.value, async () => {
  await nextTick()
  updatePosition()
  setupObserver()
})

onMounted(() => {
  nextTick(() => {
    updatePosition()
    setupObserver()
  })
})

onBeforeUnmount(() => {
  cleanupObserver()
})

const indicatorStyle = computed(() => ({
  '--vize-tabs-indicator-left': `${indicatorLeft.value}px`,
  '--vize-tabs-indicator-top': `${indicatorTop.value}px`,
  '--vize-tabs-indicator-width': `${indicatorWidth.value}px`,
  '--vize-tabs-indicator-height': `${indicatorHeight.value}px`,
}))
</script>

<template>
  <Primitive
    v-if="indicatorWidth > 0 || indicatorHeight > 0"
    :as="as"
    :as-child="asChild"
    :style="indicatorStyle"
    data-vize-tabs-indicator
  >
    <slot />
  </Primitive>
</template>
