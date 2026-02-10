<script lang="ts">
export type { CalendarHeaderProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { CalendarHeaderProps } from './types'

const { as = 'div', asChild = false } = defineProps<CalendarHeaderProps>()
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    data-vize-calendar-header
  >
    <slot />
  </Primitive>
</template>
