<script lang="ts">
export type { AccordionContentProps } from './types'
</script>

<script setup lang="ts">
import { ref, computed, toRef, onBeforeUnmount } from 'vue'
import { Primitive } from '../Primitive'
import { usePresence } from '../Presence'
import { injectAccordionRootContext } from './types'
import { injectAccordionItemContext } from './types'
import type { AccordionContentProps } from './types'

const { as = 'div', asChild = false, forceMount = false } = defineProps<AccordionContentProps>()

const rootContext = injectAccordionRootContext('AccordionContent')
const itemContext = injectAccordionItemContext('AccordionContent')

const present = computed(() => forceMount || itemContext.open.value)
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
    :id="itemContext.contentId"
    :as="as"
    :as-child="asChild"
    :ref="handleContentRef"
    role="region"
    :aria-labelledby="itemContext.triggerId"
    :hidden="!isPresent ? true : undefined"
    :data-state="itemContext.open.value ? 'open' : 'closed'"
    :data-disabled="itemContext.disabled.value ? '' : undefined"
    :data-orientation="rootContext.orientation"
    data-vize-accordion-content
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
