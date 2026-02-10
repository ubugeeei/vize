<script lang="ts">
export type { CalendarPrevProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { CalendarPrevProps } from './types'
import { injectCalendarRootContext } from './types'

const { as = 'button', asChild = false } = defineProps<CalendarPrevProps>()

const context = injectCalendarRootContext('CalendarPrev')

const isDisabled = computed(() => {
  if (context.disabled) return true
  return false
})

function handleClick() {
  if (isDisabled.value) return
  context.prevMonth()
}
</script>

<template>
  <Primitive
    :as="as || 'button'"
    :as-child="asChild"
    type="button"
    aria-label="Previous month"
    :data-disabled="isDisabled ? '' : undefined"
    :disabled="isDisabled || undefined"
    data-vize-calendar-prev
    @click="handleClick"
  >
    <slot />
  </Primitive>
</template>
