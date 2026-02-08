<script lang="ts">
export type { VirtualScrollProps, VirtualItem, VirtualScrollContext } from './types'
export { injectVirtualScrollContext, provideVirtualScrollContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { Primitive } from '../Primitive'
import type { VirtualScrollProps, VirtualItem } from './types'
import { provideVirtualScrollContext } from './types'

const {
  as = 'div',
  asChild = false,
  items,
  itemHeight,
  overscan = 3,
  horizontal = false,
} = defineProps<VirtualScrollProps>()

const scrollOffset = ref(0)
const containerRef = ref<HTMLElement>()
const containerSize = ref(0)

function getItemSize(index: number): number {
  if (typeof itemHeight === 'function') {
    return itemHeight(index)
  }
  return itemHeight
}

function getItemOffset(index: number): number {
  if (typeof itemHeight === 'number') {
    return index * itemHeight
  }
  let offset = 0
  for (let i = 0; i < index; i++) {
    offset += itemHeight(i)
  }
  return offset
}

const totalSize = computed(() => {
  if (typeof itemHeight === 'number') {
    return items.length * itemHeight
  }
  let total = 0
  for (let i = 0; i < items.length; i++) {
    total += itemHeight(i)
  }
  return total
})

const virtualItems = computed<VirtualItem[]>(() => {
  if (items.length === 0 || containerSize.value === 0) return []

  let startIndex = 0
  if (typeof itemHeight === 'number') {
    startIndex = Math.floor(scrollOffset.value / itemHeight)
  } else {
    let accumulated = 0
    for (let i = 0; i < items.length; i++) {
      accumulated += itemHeight(i)
      if (accumulated > scrollOffset.value) {
        startIndex = i
        break
      }
    }
  }

  startIndex = Math.max(0, startIndex - overscan)

  let endIndex = startIndex
  let accumulatedSize = getItemOffset(startIndex) - scrollOffset.value
  while (endIndex < items.length && accumulatedSize < containerSize.value) {
    accumulatedSize += getItemSize(endIndex)
    endIndex++
  }
  endIndex = Math.min(items.length, endIndex + overscan)

  const result: VirtualItem[] = []
  for (let i = startIndex; i < endIndex; i++) {
    result.push({
      index: i,
      start: getItemOffset(i),
      size: getItemSize(i),
      item: items[i],
    })
  }

  return result
})

function handleScroll(event: Event) {
  const el = event.currentTarget as HTMLElement
  scrollOffset.value = horizontal ? el.scrollLeft : el.scrollTop
}

let resizeObserver: ResizeObserver | undefined

onMounted(() => {
  if (containerRef.value) {
    containerSize.value = horizontal
      ? containerRef.value.clientWidth
      : containerRef.value.clientHeight

    resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        containerSize.value = horizontal
          ? entry.contentRect.width
          : entry.contentRect.height
      }
    })
    resizeObserver.observe(containerRef.value)
  }
})

onUnmounted(() => {
  resizeObserver?.disconnect()
})

provideVirtualScrollContext({
  virtualItems,
  totalSize,
  scrollOffset,
  containerRef,
})
</script>

<template>
  <Primitive
    :ref="(el) => { containerRef = el?.$el ?? el }"
    :as="as"
    :as-child="asChild"
    :tabindex="0"
    role="list"
    :aria-rowcount="items.length"
    :style="{
      overflow: 'auto',
      position: 'relative',
    }"
    @scroll="handleScroll"
  >
    <slot
      :virtual-items="virtualItems"
      :total-size="totalSize"
      :container-props="{
        style: {
          position: 'relative' as const,
          [horizontal ? 'width' : 'height']: `${totalSize}px`,
          [horizontal ? 'height' : 'width']: '100%',
        },
      }"
    />
  </Primitive>
</template>
