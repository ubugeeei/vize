<script lang="ts">
export type { VisuallyHiddenProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { VisuallyHiddenProps } from './types'

const { as = 'span', asChild = false } = defineProps<VisuallyHiddenProps>()
</script>

<template>
  <Primitive
    :as="as || 'span'"
    :as-child="asChild"
    :style="{
      position: 'absolute',
      width: '1px',
      height: '1px',
      padding: '0',
      margin: '-1px',
      overflow: 'hidden',
      clip: 'rect(0, 0, 0, 0)',
      whiteSpace: 'nowrap',
      borderWidth: '0',
    }"
  >
    <slot />
  </Primitive>
</template>
