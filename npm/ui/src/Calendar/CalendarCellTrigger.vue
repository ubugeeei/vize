<script lang="ts">
export type { CalendarCellTriggerProps } from './types'
</script>

<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue'
import { Primitive } from '../Primitive'
import type { CalendarCellTriggerProps } from './types'
import { injectCalendarRootContext } from './types'
import { isSameDay, isToday, formatFullDate } from './utils'

const { as = 'button', asChild = false, date } = defineProps<CalendarCellTriggerProps>()

const context = injectCalendarRootContext('CalendarCellTrigger')

const buttonRef = ref<HTMLElement>()

const isSelected = computed(() => context.isDateSelected(date))
const isDisabled = computed(() => context.isDateDisabled(date))
const isTodayDate = computed(() => isToday(date))
const isFocused = computed(() => isSameDay(context.focusedDate.value, date))
const isUnavailable = computed(() => context.isDateUnavailable(date))

const isOutsideMonth = computed(() => {
  const months = context.months.value
  // Check if this date's month matches any displayed month
  return !months.some(m => m.month === date.getMonth() && m.year === date.getFullYear())
})

const ariaLabel = computed(() => formatFullDate(date, context.locale))

const tabindex = computed(() => isFocused.value ? 0 : -1)

// Focus the element when it becomes the focused date
watch(() => context.focusedDate.value, async (newDate) => {
  if (isSameDay(newDate, date)) {
    await nextTick()
    buttonRef.value?.focus()
  }
})

function handleClick() {
  if (isDisabled.value) return
  context.selectDate(date)
  context.focusDate(date)
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
    data-calendar-cell-trigger
    @click="handleClick"
  >
    <slot :selected="isSelected" :disabled="isDisabled" :today="isTodayDate">
      {{ date.getDate() }}
    </slot>
  </Primitive>
</template>
