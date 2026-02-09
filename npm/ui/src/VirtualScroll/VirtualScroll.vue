<script lang="ts">
export type { VirtualScrollProps, VirtualItem, VirtualScrollContext } from './types'
export { injectVirtualScrollContext, provideVirtualScrollContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, useAttrs } from 'vue'
import { Primitive } from '../Primitive'
import type { VirtualScrollProps, VirtualItem } from './types'
import { provideVirtualScrollContext } from './types'

// Rust compiler cannot resolve imported types (VirtualScrollProps is from ./types),
// so no runtime `props` option is generated in defineComponent.
// All passed props end up in $attrs. We use useAttrs() to access them.
defineProps<VirtualScrollProps>()
const attrs = useAttrs()

const scrollOffset = ref(0)
const initContainer: HTMLElement | undefined = undefined
const containerRef = ref(initContainer)
const containerSize = ref(0)

// Helper: attrs use kebab-case when props are not declared
function attr(name: string): unknown {
  if (name in attrs) return attrs[name]
  const kebab = name.replace(/[A-Z]/g, (m: string) => '-' + m.toLowerCase())
  return attrs[kebab]
}

const itemList = computed(() => (attr('items') ?? []) as unknown[])
const componentAs = computed(() => (attr('as') ?? 'div') as string)
const componentAsChild = computed(() => !!attr('asChild'))
const isHorizontal = computed(() => !!attr('horizontal'))

function getItemHeight(): number | ((index: number) => number) {
  return (attr('itemHeight') ?? 40) as number | ((index: number) => number)
}

function getItemSize(index: number): number {
  const ih = getItemHeight()
  if (typeof ih === 'function') {
    return ih(index)
  }
  return ih
}

function getItemOffset(index: number): number {
  const ih = getItemHeight()
  if (typeof ih === 'number') {
    return index * ih
  }
  let offset = 0
  for (let i = 0; i < index; i++) {
    offset += getItemSize(i)
  }
  return offset
}

const totalSize = computed(() => {
  const list = itemList.value
  const ih = getItemHeight()
  if (typeof ih === 'number') {
    return list.length * ih
  }
  let total = 0
  for (let i = 0; i < list.length; i++) {
    total += getItemSize(i)
  }
  return total
})

function computeVirtualItems(): VirtualItem[] {
  const list = itemList.value
  const os = (attr('overscan') ?? 3) as number
  if (list.length === 0 || containerSize.value === 0) return []

  const ih = getItemHeight()
  let startIndex = 0
  if (typeof ih === 'number') {
    startIndex = Math.floor(scrollOffset.value / ih)
  } else {
    let accumulated = 0
    for (let i = 0; i < list.length; i++) {
      accumulated += getItemSize(i)
      if (accumulated > scrollOffset.value) {
        startIndex = i
        break
      }
    }
  }

  startIndex = Math.max(0, startIndex - os)

  let endIndex = startIndex
  let accumulatedSize = getItemOffset(startIndex) - scrollOffset.value
  while (endIndex < list.length && accumulatedSize < containerSize.value) {
    accumulatedSize += getItemSize(endIndex)
    endIndex++
  }
  endIndex = Math.min(list.length, endIndex + os)

  const result: VirtualItem[] = []
  for (let i = startIndex; i < endIndex; i++) {
    result.push({
      index: i,
      start: getItemOffset(i),
      size: getItemSize(i),
      item: list[i],
    })
  }

  return result
}

const virtualItems = computed(() => computeVirtualItems())

function handleScroll(event: Event) {
  const el: HTMLElement = event.currentTarget
  scrollOffset.value = isHorizontal.value ? el.scrollLeft : el.scrollTop
}

let resizeObserver: ResizeObserver | undefined

onMounted(() => {
  if (containerRef.value) {
    containerSize.value = isHorizontal.value
      ? containerRef.value.clientWidth
      : containerRef.value.clientHeight

    resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        containerSize.value = isHorizontal.value
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
    :as="componentAs"
    :as-child="componentAsChild"
    :tabindex="0"
    role="list"
    :aria-rowcount="itemList.length"
    :style="{
      overflow: 'auto',
      position: 'relative',
    }"
    @scroll="handleScroll"
  >
    <slot
      :virtualItems="virtualItems"
      :totalSize="totalSize"
      :containerProps="{
        style: {
          position: 'relative',
          [isHorizontal ? 'width' : 'height']: totalSize + 'px',
          [isHorizontal ? 'height' : 'width']: '100%',
        },
      }"
    />
  </Primitive>
</template>
