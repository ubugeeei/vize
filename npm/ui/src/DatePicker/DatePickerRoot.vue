<script lang="ts">
export type { DatePickerRootProps, DatePickerRootContext } from './types'
export { injectDatePickerRootContext, provideDatePickerRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useId } from '../shared'
import type { DatePickerRootProps } from './types'
import { provideDatePickerRootContext } from './types'
import type { DateValue } from '../Calendar/types'

const {
  modelValue,
  defaultValue,
  disabled = false,
  readonly: readonlyProp = false,
  locale = 'en-US',
  formatDate: formatDateProp,
} = defineProps<DatePickerRootProps>()

const emit = defineEmits<{
  'update:modelValue': [value: DateValue]
  'update:open': [value: boolean]
}>()

const contentId = useId()
const triggerRef = ref<HTMLElement>()
const open = ref(false)

const internalValue = ref<DateValue | undefined>(defaultValue)

const selectedDate = computed<DateValue | undefined>(() => {
  if (modelValue !== undefined) return modelValue
  return internalValue.value
})

function defaultFormatDate(date: DateValue): string {
  return date.toLocaleDateString(locale, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })
}

const formatDate = formatDateProp ?? defaultFormatDate

function onDateSelect(date: DateValue) {
  if (disabled || readonlyProp) return
  internalValue.value = date
  emit('update:modelValue', date)
  onOpenChange(false)
}

function onOpenChange(value: boolean) {
  if (disabled) return
  open.value = value
  emit('update:open', value)
}

provideDatePickerRootContext({
  modelValue: selectedDate,
  open,
  disabled,
  readonly: readonlyProp,
  locale,
  formatDate,
  onDateSelect,
  onOpenChange,
  triggerRef,
  contentId,
})
</script>

<template>
  <slot :open="open" :model-value="selectedDate" />
</template>
