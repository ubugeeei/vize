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
    :as="as || 'th'"
    :as-child="asChild"
    scope="col"
    data-vize-calendar-grid-head-cell
  >
    <slot />
  </Primitive>
</template>
