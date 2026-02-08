<script lang="ts">
export type { CalendarGridBodyProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { CalendarGridBodyProps } from './types'

const { as = 'tbody', asChild = false } = defineProps<CalendarGridBodyProps>()
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    data-calendar-grid-body
  >
    <slot />
  </Primitive>
</template>
