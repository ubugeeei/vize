<script lang="ts">
export type { DatePickerInputProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { DatePickerInputProps } from './types'
import { injectDatePickerRootContext } from './types'

const { as = 'input', asChild = false, placeholder } = defineProps<DatePickerInputProps>()

const context = injectDatePickerRootContext('DatePickerInput')

const displayValue = computed(() => {
  const date = context.modelValue.value
  if (!date) return ''
  return context.formatDate(date)
})

const placeholderText = computed(() => placeholder ?? 'Select a date')
</script>

<template>
  <Primitive
    :as="as || 'input'"
    :as-child="asChild"
    type="text"
    readonly
    :value="displayValue"
    :placeholder="placeholderText"
    aria-label="Selected date"
    :aria-describedby="context.contentId"
    :data-disabled="context.disabled ? '' : undefined"
    :data-state="context.open.value ? 'open' : 'closed'"
    :disabled="context.disabled || undefined"
    data-vize-datepicker-input
  >
    <slot :display-value="displayValue" />
  </Primitive>
</template>
