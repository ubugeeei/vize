<script lang="ts">
export type { PopoverArrowProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import { injectPopoverRootContext } from './types'
import type { PopoverArrowProps } from './types'

const { as = 'svg', asChild = false, width = 10, height = 5 } = defineProps<PopoverArrowProps>()

const context = injectPopoverRootContext('PopoverArrow')
</script>

<template>
  <Primitive
    :as="as || 'svg'"
    :as-child="asChild"
    :width="width"
    :height="height"
    :viewBox="`0 0 ${width} ${height}`"
    fill="none"
    data-vize-popover-arrow
  >
    <slot>
      <polygon :points="`0,${height} ${width / 2},0 ${width},${height}`" fill="currentColor" />
    </slot>
  </Primitive>
</template>
