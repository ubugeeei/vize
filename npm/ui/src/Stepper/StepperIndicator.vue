<script lang="ts">
export type { StepperIndicatorProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { StepperIndicatorProps } from './types'
import { injectStepperItemContext } from './types'

const { as = 'span', asChild = false } = defineProps<StepperIndicatorProps>()

const itemContext = injectStepperItemContext('StepperIndicator')
</script>

<template>
  <Primitive
    :as="as || 'span'"
    :as-child="asChild"
    :data-state="itemContext.state.value"
    data-vize-stepper-indicator
  >
    <slot :step="itemContext.step" :state="itemContext.state.value">
      {{ itemContext.step }}
    </slot>
  </Primitive>
</template>
