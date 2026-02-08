<script lang="ts">
export type { CalendarGridHeadProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { CalendarGridHeadProps } from './types'

const { as = 'thead', asChild = false } = defineProps<CalendarGridHeadProps>()
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    aria-hidden="true"
    data-calendar-grid-head
  >
    <slot />
  </Primitive>
</template>
