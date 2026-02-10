<script lang="ts">
export type { TooltipTriggerProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { injectTooltipRootContext } from './types'
import { injectTooltipProviderContext } from './types'
import type { TooltipTriggerProps } from './types'

const { as = 'button', asChild = false } = defineProps<TooltipTriggerProps>()

const context = injectTooltipRootContext('TooltipTrigger')
const provider = injectTooltipProviderContext('TooltipTrigger')

const dataState = computed(() => {
  if (!context.open.value) return 'closed'
  return provider.isOpenDelayed.value ? 'delayed-open' : 'instant-open'
})

function setTriggerRef(el: HTMLElement | undefined) {
  if (el instanceof HTMLElement) {
    context.triggerRef.value = el
  }
}

function handlePointerEnter() {
  context.onOpenChange(true)
}

function handlePointerLeave() {
  if (!provider.disableClosingTrigger) {
    context.onOpenChange(false)
  }
}

function handleFocus() {
  context.onOpenChange(true)
}

function handleBlur() {
  context.onOpenChange(false)
}
</script>

<template>
  <Primitive
    :ref="setTriggerRef"
    :as="as || 'button'"
    :as-child="asChild"
    :aria-describedby="context.open.value ? context.contentId : undefined"
    :data-state="dataState"
    data-vize-tooltip-trigger
    @pointerenter="handlePointerEnter"
    @pointerleave="handlePointerLeave"
    @focus="handleFocus"
    @blur="handleBlur"
  >
    <slot />
  </Primitive>
</template>
