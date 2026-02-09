<script lang="ts">
export type { CalendarCellProps } from './types'
</script>

<script setup lang="ts">
import { computed, useAttrs } from 'vue'
import { Primitive } from '../Primitive'
import type { CalendarCellProps, DateValue } from './types'
import { injectCalendarRootContext } from './types'

// Rust compiler cannot resolve imported types, so date prop ends up in $attrs
defineProps<CalendarCellProps>()
const attrs = useAttrs()

const context = injectCalendarRootContext('CalendarCell')

// Access date from attrs since Rust compiler doesn't generate props option
const dateValue = computed(() => attrs.date as DateValue)

const isSelected = computed(() => context.isDateSelected(dateValue.value))
const isDisabled = computed(() => context.isDateDisabled(dateValue.value))
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="gridcell"
    :aria-selected="isSelected || undefined"
    :aria-disabled="isDisabled || undefined"
    :data-disabled="isDisabled ? '' : undefined"
    data-vize-calendar-cell
  >
    <slot />
  </Primitive>
</template>
