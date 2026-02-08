<script lang="ts">
export type { CalendarGridHeadCellProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { CalendarGridHeadCellProps } from './types'

const { as = 'th', asChild = false } = defineProps<CalendarGridHeadCellProps>()
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    scope="col"
    data-calendar-grid-head-cell
  >
    <slot />
  </Primitive>
</template>
