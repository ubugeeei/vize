<script lang="ts">
export type { PopoverTriggerProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import { injectPopoverRootContext } from './types'
import type { PopoverTriggerProps } from './types'

const { as = 'button', asChild = false } = defineProps<PopoverTriggerProps>()

const context = injectPopoverRootContext('PopoverTrigger')

function setTriggerRef(el: HTMLElement | undefined) {
  if (el instanceof HTMLElement) {
    context.triggerRef.value = el
  }
}

function handleClick() {
  context.onOpenChange(!context.open.value)
}
</script>

<template>
  <Primitive
    :ref="setTriggerRef"
    :as="as || 'button'"
    :as-child="asChild"
    type="button"
    aria-haspopup="dialog"
    :aria-expanded="context.open.value"
    :aria-controls="context.contentId"
    :data-state="context.open.value ? 'open' : 'closed'"
    data-vize-popover-trigger
    @click="handleClick"
  >
    <slot />
  </Primitive>
</template>
