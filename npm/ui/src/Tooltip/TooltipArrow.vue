<script lang="ts">
export type { TooltipArrowProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import { injectTooltipRootContext } from './types'
import type { TooltipArrowProps } from './types'

const { as = 'svg', asChild = false, width = 10, height = 5 } = defineProps<TooltipArrowProps>()

const context = injectTooltipRootContext('TooltipArrow')
</script>

<template>
  <Primitive
    :as="as || 'svg'"
    :as-child="asChild"
    :width="width"
    :height="height"
    :viewBox="`0 0 ${width} ${height}`"
    fill="none"
    data-vize-tooltip-arrow
  >
    <slot>
      <polygon :points="`0,${height} ${width / 2},0 ${width},${height}`" fill="currentColor" />
    </slot>
  </Primitive>
</template>
