<script lang="ts">
export type { SeparatorProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { SeparatorProps } from './types'

const { as = 'div', asChild = false, orientation = 'horizontal', decorative = false } = defineProps<SeparatorProps>()

const ariaOrientation = computed(() => orientation === 'vertical' ? 'vertical' : undefined)
const role = computed(() => decorative ? 'none' : 'separator')
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    :role="role"
    :aria-orientation="ariaOrientation"
    :data-orientation="orientation"
    data-vize-separator
  >
    <slot />
  </Primitive>
</template>
