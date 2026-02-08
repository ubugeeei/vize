<script lang="ts">
export type { CalendarCellProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { CalendarCellProps } from './types'
import { injectCalendarRootContext } from './types'

const { as = 'td', asChild = false, date } = defineProps<CalendarCellProps>()

const context = injectCalendarRootContext('CalendarCell')

const isSelected = computed(() => context.isDateSelected(date))
const isDisabled = computed(() => context.isDateDisabled(date))
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="gridcell"
    :aria-selected="isSelected || undefined"
    :aria-disabled="isDisabled || undefined"
    :data-disabled="isDisabled ? '' : undefined"
    data-calendar-cell
  >
    <slot />
  </Primitive>
</template>
