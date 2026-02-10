<script lang="ts">
export type { CalendarNextProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { CalendarNextProps } from './types'
import { injectCalendarRootContext } from './types'
const { as = 'button', asChild = false } = defineProps<CalendarNextProps>()

const context = injectCalendarRootContext('CalendarNext')

const isDisabled = computed(() => {
  if (context.disabled) return true
  return false
})

function handleClick() {
  if (isDisabled.value) return
  context.nextMonth()
}
</script>

<template>
  <Primitive
    :as="as || 'button'"
    :as-child="asChild"
    type="button"
    aria-label="Next month"
    :data-disabled="isDisabled ? '' : undefined"
    :disabled="isDisabled || undefined"
    data-vize-calendar-next
    @click="handleClick"
  >
    <slot />
  </Primitive>
</template>
