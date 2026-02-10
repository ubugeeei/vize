<script lang="ts">
export type { StepperItemProps, StepperItemContext, StepState } from './types'
export { injectStepperItemContext, provideStepperItemContext } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { useId } from '../shared'
import type { StepperItemProps, StepState } from './types'
import { injectStepperRootContext, provideStepperItemContext } from './types'

const {
  as = 'div',
  asChild = false,
  step,
  disabled = false,
  completed,
} = defineProps<StepperItemProps>()

const rootContext = injectStepperRootContext('StepperItem')

const triggerId = useId()

const state = computed<StepState>(() => {
  if (completed) return 'completed'
  if (rootContext.currentStep.value === step) return 'active'
  if (rootContext.currentStep.value > step) return 'completed'
  return 'upcoming'
})

provideStepperItemContext({
  step,
  state,
  disabled,
  triggerId,
})
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    :data-state="state"
    :data-disabled="disabled ? '' : undefined"
    data-vize-stepper-item
    :aria-current="state === 'active' ? 'step' : undefined"
  >
    <slot :state="state" :step="step" />
  </Primitive>
</template>
