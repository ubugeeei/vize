<script lang="ts">
export type { CollapsibleContentProps } from './types'
</script>

<script setup lang="ts">
import { ref, computed, toRef, onBeforeUnmount } from 'vue'
import { Primitive } from '../Primitive'
import { usePresence } from '../Presence'
import { injectCollapsibleRootContext } from './types'
import type { CollapsibleContentProps } from './types'

const { as = 'div', asChild = false, forceMount = false } = defineProps<CollapsibleContentProps>()

const context = injectCollapsibleRootContext('CollapsibleContent')

const present = computed(() => forceMount || context.open.value)
const { isPresent, ref: presenceRef, onAnimationStart, onAnimationEnd } = usePresence(toRef(present))

const contentRef = ref<HTMLElement>()
const contentWidth = ref(0)
const contentHeight = ref(0)

let resizeObserver: ResizeObserver | undefined

function handleContentRef(el: any) {
  const node = el?.$el ?? el
  presenceRef(node)
  if (node) {
    contentRef.value = node
    observeSize(node)
  }
}

function observeSize(element: HTMLElement) {
  cleanupObserver()
  resizeObserver = new ResizeObserver((entries) => {
    for (const entry of entries) {
      const borderBoxSize = entry.borderBoxSize[0]
      if (borderBoxSize) {
        contentWidth.value = borderBoxSize.inlineSize
        contentHeight.value = borderBoxSize.blockSize
      } else {
        const rect = element.getBoundingClientRect()
        contentWidth.value = rect.width
        contentHeight.value = rect.height
      }
    }
  })
  resizeObserver.observe(element)
}

function cleanupObserver() {
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = undefined
  }
}

onBeforeUnmount(() => {
  cleanupObserver()
})
</script>

<template>
  <Primitive
    v-if="isPresent"
    :id="context.contentId"
    :as="as"
    :as-child="asChild"
    :ref="handleContentRef"
    role="region"
    :hidden="!isPresent ? true : undefined"
    :data-state="context.open.value ? 'open' : 'closed'"
    :data-disabled="context.disabled ? '' : undefined"
    :style="{
      '--vize-collapsible-content-height': `${contentHeight}px`,
      '--vize-collapsible-content-width': `${contentWidth}px`,
    }"
    @animationstart="onAnimationStart"
    @animationend="onAnimationEnd"
  >
    <slot />
  </Primitive>
</template>
