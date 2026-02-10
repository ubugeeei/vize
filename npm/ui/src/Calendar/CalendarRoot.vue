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
import { ref, computed, watch, useAttrs } from 'vue'
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

// Rust compiler cannot resolve imported types (CalendarRootProps is from ./types),
// so no runtime `props` option is generated in defineComponent.
// All passed props end up in $attrs. We use useAttrs() to access them.
defineProps<CalendarRootProps>()
const attrs = useAttrs()

// Helper: attrs use kebab-case when props are not declared
function attr(name: string): unknown {
  if (name in attrs) return attrs[name]
  const kebab = name.replace(/[A-Z]/g, (m: string) => '-' + m.toLowerCase())
  return attrs[kebab]
}

const emit = defineEmits<{
  'update:modelValue': [value: DateValue | DateValue[]]
}>()

const componentAs = computed(() => (attr('as') ?? 'div') as string)
const componentAsChild = computed(() => !!attr('asChild'))
const isDisabled = computed(() => !!attr('disabled'))
const isReadonly = computed(() => !!attr('readonly'))
const isMultiple = computed(() => !!attr('multiple'))
const resolvedLocale = computed(() => (attr('locale') ?? 'en-US') as string)
const resolvedWeekStartsOn = computed(() => (attr('weekStartsOn') ?? 1) as number)
const resolvedNumberOfMonths = computed(() => (attr('numberOfMonths') ?? 1) as number)
const resolvedFixedWeeks = computed(() => !!attr('fixedWeeks'))

const direction = useDirection(computed(() => attr('dir') as string | undefined))

// Internal selection state
const initDefault = attr('defaultValue')
const initArray: DateValue[] = initDefault !== undefined
  ? (Array.isArray(initDefault) ? initDefault : [initDefault]) as DateValue[]
  : []
const internalValue = ref(initArray)

const selectedDates = computed<DateValue[]>(() => {
  const mv = attr('modelValue')
  if (mv !== undefined) {
    return Array.isArray(mv) ? mv : [mv]
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
  const lastMonth = addMonths(firstMonth, resolvedNumberOfMonths.value - 1)
  const focusedInRange =
    (isSameMonth(newFocused, firstMonth) || isAfterDay(newFocused, startOfMonth(firstMonth)))
    && (isSameMonth(newFocused, lastMonth) || isBeforeDay(newFocused, endOfMonth(lastMonth)))

  if (!focusedInRange) {
    displayMonth.value = startOfMonth(newFocused)
  }
})

function isDateDisabledFn(date: DateValue): boolean {
  if (isDisabled.value) return true
  const minD = attr('minDate') as DateValue | undefined
  const maxD = attr('maxDate') as DateValue | undefined
  if (minD && isBeforeDay(date, minD)) return true
  if (maxD && isAfterDay(date, maxD)) return true
  const customFn = attr('isDateDisabled') as ((d: DateValue) => boolean) | undefined
  if (customFn && customFn(date)) return true
  return false
}

function isDateUnavailableFn(date: DateValue): boolean {
  const customFn = attr('isDateUnavailable') as ((d: DateValue) => boolean) | undefined
  if (customFn) return customFn(date)
  return false
}

function isDateSelected(date: DateValue): boolean {
  return selectedDates.value.some(d => isSameDay(d, date))
}

function selectDate(date: DateValue) {
  if (isDisabled.value || isReadonly.value) return
  if (isDateDisabledFn(date)) return
  if (isDateUnavailableFn(date)) return

  if (isMultiple.value) {
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
const weekDays = computed(() => getWeekDayNames(resolvedLocale.value, resolvedWeekStartsOn.value))

// Build month grids
const months = computed<CalendarMonth[]>(() => {
  const result: CalendarMonth[] = []

  for (let i = 0; i < resolvedNumberOfMonths.value; i++) {
    const monthDate = addMonths(displayMonth.value, i)
    const year = monthDate.getFullYear()
    const month = monthDate.getMonth()
    const grid = getMonthGrid(year, month, resolvedWeekStartsOn.value, resolvedFixedWeeks.value)

    const weeks = grid.map(week =>
      week.map(date => ({
        date,
        isToday: isToday(date),
        isSelected: isDateSelected(date),
        isDisabled: isDateDisabledFn(date),
        isUnavailable: isDateUnavailableFn(date),
        isOutsideMonth: date.getMonth() !== month,
      })),
    )

    result.push({ year, month, weeks })
  }

  return result
})

function handleKeydown(event: KeyboardEvent) {
  if (isDisabled.value || isReadonly.value) return

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
      focusedDate.value = startOfWeek(focusedDate.value, resolvedWeekStartsOn.value)
      break
    case kbd.END: {
      const weekEnd = addDays(startOfWeek(focusedDate.value, resolvedWeekStartsOn.value), 6)
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
  locale: resolvedLocale.value,
  disabled: isDisabled.value,
  readonly: isReadonly.value,
  multiple: isMultiple.value,
  dir: direction,
  isDateDisabled: isDateDisabledFn,
  isDateUnavailable: isDateUnavailableFn,
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
    :as="componentAs"
    :as-child="componentAsChild"
    role="application"
    :data-disabled="isDisabled ? '' : undefined"
    :data-readonly="isReadonly ? '' : undefined"
    data-vize-calendar
    :dir="direction"
    @keydown="handleKeydown"
  >
    <slot
      :months="months"
      :weekDays="weekDays"
      :focusedDate="focusedDate"
    />
  </Primitive>
</template>
