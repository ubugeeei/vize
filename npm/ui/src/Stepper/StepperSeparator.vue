<script lang="ts">
export type { StepperSeparatorProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { StepperSeparatorProps } from './types'
import { injectStepperRootContext, injectStepperItemContext } from './types'

const { as = 'div', asChild = false } = defineProps<StepperSeparatorProps>()

const rootContext = injectStepperRootContext('StepperSeparator')
const itemContext = injectStepperItemContext('StepperSeparator')

const separatorState = computed(() => {
  return rootContext.currentStep.value > itemContext.step ? 'completed' : 'upcoming'
})
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="separator"
    :data-state="separatorState"
    :data-orientation="rootContext.orientation"
    data-vize-stepper-separator
  >
    <slot />
  </Primitive>
</template>
