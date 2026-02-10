<script lang="ts">
export type { CalendarGridProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { CalendarGridProps } from './types'
import { injectCalendarRootContext } from './types'
import { formatMonth } from './utils'

const { as = 'table', asChild = false } = defineProps<CalendarGridProps>()

const context = injectCalendarRootContext('CalendarGrid')

const ariaLabel = computed(() => {
  const months = context.months.value
  if (months.length === 0) return ''
  const first = months[0]
  return formatMonth(new Date(first.year, first.month, 1), context.locale)
})
</script>

<template>
  <Primitive
    :as="as || 'table'"
    :as-child="asChild"
    role="grid"
    :aria-label="ariaLabel"
    data-vize-calendar-grid
  >
    <slot />
  </Primitive>
</template>
