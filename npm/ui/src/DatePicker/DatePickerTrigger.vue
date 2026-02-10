<script lang="ts">
export type { DatePickerTriggerProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { DatePickerTriggerProps } from './types'
import { injectDatePickerRootContext } from './types'

const { as = 'button', asChild = false } = defineProps<DatePickerTriggerProps>()

const context = injectDatePickerRootContext('DatePickerTrigger')

function setTriggerRef(el: any) {
  const element = el?.$el ?? el
  if (element instanceof HTMLElement) {
    context.triggerRef.value = element
  }
}

function handleClick() {
  if (context.disabled) return
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
    :aria-controls="context.open.value ? context.contentId : undefined"
    :data-state="context.open.value ? 'open' : 'closed'"
    :data-disabled="context.disabled ? '' : undefined"
    :disabled="context.disabled || undefined"
    data-vize-datepicker-trigger
    @click="handleClick"
  >
    <slot />
  </Primitive>
</template>
