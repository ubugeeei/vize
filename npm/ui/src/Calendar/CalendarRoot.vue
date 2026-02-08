<script lang="ts">
export type {
  CalendarRootProps,
  CalendarRootContext,
  CalendarDay,
  CalendarMonth,
  DateValue,
} from './types'
export { injectCalendarRootContext, provideCalendarRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { Primitive } from '../Primitive'
import { useDirection, kbd } from '../shared'
import type { CalendarRootProps, CalendarMonth, DateValue } from './types'
import { provideCalendarRootContext } from './types'
import {
  isSameDay,
  isSameMonth,
  isToday,
  isBeforeDay,
  isAfterDay,
  addDays,
  addMonths,
  startOfMonth,
  endOfMonth,
  startOfWeek,
  getMonthGrid,
  getWeekDayNames,
} from './utils'

const {
  as = 'div',
  asChild = false,
  modelValue,
  defaultValue,
  minDate,
  maxDate,
  disabled = false,
  readonly: readonlyProp = false,
  multiple = false,
  dir: dirProp,
  locale = 'en-US',
  weekStartsOn = 1,
  numberOfMonths = 1,
  fixedWeeks = false,
  isDateDisabled: isDateDisabledProp,
  isDateUnavailable: isDateUnavailableProp,
} = defineProps<CalendarRootProps>()

const emit = defineEmits<{
  'update:modelValue': [value: DateValue | DateValue[]]
}>()

const direction = useDirection(computed(() => dirProp))

// Internal selection state
const internalValue = ref<DateValue[]>(
  defaultValue !== undefined
    ? (Array.isArray(defaultValue) ? defaultValue : [defaultValue])
    : [],
)

const selectedDates = computed<DateValue[]>(() => {
  if (modelValue !== undefined) {
    return Array.isArray(modelValue) ? modelValue : [modelValue]
  }
  return internalValue.value
})

// Focused date for keyboard navigation
const focusedDate = ref<DateValue>(
  selectedDates.value.length > 0
    ? new Date(selectedDates.value[0])
    : new Date(),
)

// Current display month (first month being shown)
const displayMonth = ref<DateValue>(new Date(focusedDate.value))

// Sync displayMonth when focusedDate changes
watch(focusedDate, (newFocused) => {
  // Check if focused date is visible in current range
  const firstMonth = displayMonth.value
  const lastMonth = addMonths(firstMonth, numberOfMonths - 1)
  const focusedInRange =
    (isSameMonth(newFocused, firstMonth) || isAfterDay(newFocused, startOfMonth(firstMonth)))
    && (isSameMonth(newFocused, lastMonth) || isBeforeDay(newFocused, endOfMonth(lastMonth)))

  if (!focusedInRange) {
    displayMonth.value = startOfMonth(newFocused)
  }
})

function isDateDisabled(date: DateValue): boolean {
  if (disabled) return true
  if (minDate && isBeforeDay(date, minDate)) return true
  if (maxDate && isAfterDay(date, maxDate)) return true
  if (isDateDisabledProp && isDateDisabledProp(date)) return true
  return false
}

function isDateUnavailable(date: DateValue): boolean {
  if (isDateUnavailableProp) return isDateUnavailableProp(date)
  return false
}

function isDateSelected(date: DateValue): boolean {
  return selectedDates.value.some(d => isSameDay(d, date))
}

function selectDate(date: DateValue) {
  if (disabled || readonlyProp) return
  if (isDateDisabled(date)) return
  if (isDateUnavailable(date)) return

  if (multiple) {
    const existing = selectedDates.value.findIndex(d => isSameDay(d, date))
    let next: DateValue[]
    if (existing >= 0) {
      next = selectedDates.value.filter((_, i) => i !== existing)
    } else {
      next = [...selectedDates.value, date]
    }
    internalValue.value = next
    emit('update:modelValue', next)
  } else {
    internalValue.value = [date]
    emit('update:modelValue', date)
  }
}

function focusDate(date: DateValue) {
  focusedDate.value = date
}

function nextMonth() {
  displayMonth.value = addMonths(displayMonth.value, 1)
}

function prevMonth() {
  displayMonth.value = addMonths(displayMonth.value, -1)
}

function nextYear() {
  displayMonth.value = addMonths(displayMonth.value, 12)
}

function prevYear() {
  displayMonth.value = addMonths(displayMonth.value, -12)
}

// Week day names
const weekDays = computed(() => getWeekDayNames(locale, weekStartsOn))

// Build month grids
const months = computed<CalendarMonth[]>(() => {
  const result: CalendarMonth[] = []

  for (let i = 0; i < numberOfMonths; i++) {
    const monthDate = addMonths(displayMonth.value, i)
    const year = monthDate.getFullYear()
    const month = monthDate.getMonth()
    const grid = getMonthGrid(year, month, weekStartsOn, fixedWeeks)

    const weeks = grid.map(week =>
      week.map(date => ({
        date,
        isToday: isToday(date),
        isSelected: isDateSelected(date),
        isDisabled: isDateDisabled(date),
        isUnavailable: isDateUnavailable(date),
        isOutsideMonth: date.getMonth() !== month,
      })),
    )

    result.push({ year, month, weeks })
  }

  return result
})

function handleKeydown(event: KeyboardEvent) {
  if (disabled || readonlyProp) return

  const isRtl = direction.value === 'rtl'
  let handled = true

  switch (event.key) {
    case kbd.ARROW_RIGHT:
      focusedDate.value = addDays(focusedDate.value, isRtl ? -1 : 1)
      break
    case kbd.ARROW_LEFT:
      focusedDate.value = addDays(focusedDate.value, isRtl ? 1 : -1)
      break
    case kbd.ARROW_DOWN:
      focusedDate.value = addDays(focusedDate.value, 7)
      break
    case kbd.ARROW_UP:
      focusedDate.value = addDays(focusedDate.value, -7)
      break
    case kbd.PAGE_DOWN:
      if (event.shiftKey) {
        focusedDate.value = addMonths(focusedDate.value, 12)
      } else {
        focusedDate.value = addMonths(focusedDate.value, 1)
      }
      break
    case kbd.PAGE_UP:
      if (event.shiftKey) {
        focusedDate.value = addMonths(focusedDate.value, -12)
      } else {
        focusedDate.value = addMonths(focusedDate.value, -1)
      }
      break
    case kbd.HOME:
      focusedDate.value = startOfWeek(focusedDate.value, weekStartsOn)
      break
    case kbd.END: {
      const weekEnd = addDays(startOfWeek(focusedDate.value, weekStartsOn), 6)
      focusedDate.value = weekEnd
      break
    }
    case kbd.ENTER:
    case kbd.SPACE:
      event.preventDefault()
      selectDate(focusedDate.value)
      return
    default:
      handled = false
      break
  }

  if (handled) {
    event.preventDefault()
  }
}

provideCalendarRootContext({
  modelValue: selectedDates,
  focusedDate,
  months,
  weekDays,
  locale,
  disabled,
  readonly: readonlyProp,
  multiple,
  dir: direction,
  isDateDisabled,
  isDateUnavailable,
  isDateSelected,
  selectDate,
  focusDate,
  nextMonth,
  prevMonth,
  nextYear,
  prevYear,
})
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="application"
    :data-disabled="disabled ? '' : undefined"
    :data-readonly="readonlyProp ? '' : undefined"
    :dir="direction"
    @keydown="handleKeydown"
  >
    <slot
      :months="months"
      :week-days="weekDays"
      :focused-date="focusedDate"
    />
  </Primitive>
</template>
