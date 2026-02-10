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
    :as="as || 'tbody'"
    :as-child="asChild"
    data-vize-calendar-grid-body
  >
    <slot />
  </Primitive>
</template>
