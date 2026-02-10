<script lang="ts">
export type { CalendarCellTriggerProps } from './types'
</script>

<script setup lang="ts">
import { computed, ref, watch, nextTick, useAttrs } from 'vue'
import { Primitive } from '../Primitive'
import type { CalendarCellTriggerProps, DateValue } from './types'
import { injectCalendarRootContext } from './types'
import { isSameDay, isToday, formatFullDate } from './utils'

// Rust compiler cannot resolve imported types, so date prop ends up in $attrs
defineProps<CalendarCellTriggerProps>()
const attrs = useAttrs()

const context = injectCalendarRootContext('CalendarCellTrigger')

const buttonRef = ref<HTMLElement>()

// Access date from attrs since Rust compiler doesn't generate props option
const dateValue = computed(() => attrs.date as DateValue)

const isSelected = computed(() => context.isDateSelected(dateValue.value))
const isDisabled = computed(() => context.isDateDisabled(dateValue.value))
const isTodayDate = computed(() => isToday(dateValue.value))
const isFocused = computed(() => isSameDay(context.focusedDate.value, dateValue.value))
const isUnavailable = computed(() => context.isDateUnavailable(dateValue.value))

const isOutsideMonth = computed(() => {
  const months = context.months.value
  const d = dateValue.value
  // Check if this date's month matches any displayed month
  return !months.some(m => m.month === d.getMonth() && m.year === d.getFullYear())
})

const ariaLabel = computed(() => formatFullDate(dateValue.value, context.locale))

const tabindex = computed(() => isFocused.value ? 0 : -1)

// Focus the element when it becomes the focused date
watch(() => context.focusedDate.value, async (newDate) => {
  if (isSameDay(newDate, dateValue.value)) {
    await nextTick()
    buttonRef.value?.focus()
  }
})

function handleClick() {
  if (isDisabled.value) return
  context.selectDate(dateValue.value)
  context.focusDate(dateValue.value)
}

function setRef(el: any) {
  buttonRef.value = el?.$el ?? el
}
</script>

<template>
  <Primitive
    :ref="setRef"
    :as="as"
    :as-child="asChild"
    type="button"
    :tabindex="tabindex"
    :aria-label="ariaLabel"
    :aria-selected="isSelected || undefined"
    :aria-disabled="isDisabled || undefined"
    :disabled="isDisabled || undefined"
    :data-state="isSelected ? 'selected' : 'unselected'"
    :data-today="isTodayDate ? '' : undefined"
    :data-disabled="isDisabled ? '' : undefined"
    :data-unavailable="isUnavailable ? '' : undefined"
    :data-outside-month="isOutsideMonth ? '' : undefined"
    :data-focused="isFocused ? '' : undefined"
    data-vize-calendar-cell-trigger
    @click="handleClick"
  >
    <slot :selected="isSelected" :disabled="isDisabled" :today="isTodayDate">
      {{ dateValue.getDate() }}
    </slot>
  </Primitive>
</template>
