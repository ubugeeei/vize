<script lang="ts">
export type { DialogTriggerProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import { injectDialogRootContext } from './types'
import type { DialogTriggerProps } from './types'

const { as = 'button', asChild = false } = defineProps<DialogTriggerProps>()

const context = injectDialogRootContext('DialogTrigger')

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
    data-vize-dialog-trigger
    @click="handleClick"
  >
    <slot />
  </Primitive>
</template>
