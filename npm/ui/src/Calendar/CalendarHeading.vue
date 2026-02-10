<script lang="ts">
export type { CalendarHeadingProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { CalendarHeadingProps } from './types'
import { injectCalendarRootContext } from './types'
import { formatMonth } from './utils'

const { as = 'div', asChild = false } = defineProps<CalendarHeadingProps>()

const context = injectCalendarRootContext('CalendarHeading')

const headingText = computed(() => {
  const months = context.months.value
  if (months.length === 0) return ''

  if (months.length === 1) {
    return formatMonth(new Date(months[0].year, months[0].month, 1), context.locale)
  }

  const first = months[0]
  const last = months[months.length - 1]
  const firstLabel = formatMonth(new Date(first.year, first.month, 1), context.locale)
  const lastLabel = formatMonth(new Date(last.year, last.month, 1), context.locale)
  return `${firstLabel} - ${lastLabel}`
})
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    aria-live="polite"
    role="heading"
    aria-level="2"
    data-vize-calendar-heading
  >
    <slot :heading="headingText">
      {{ headingText }}
    </slot>
  </Primitive>
</template>
