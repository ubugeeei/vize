<script lang="ts">
export type { StepperTriggerProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { StepperTriggerProps } from './types'
import { injectStepperRootContext, injectStepperItemContext } from './types'

const { as = 'button', asChild = false } = defineProps<StepperTriggerProps>()

const rootContext = injectStepperRootContext('StepperTrigger')
const itemContext = injectStepperItemContext('StepperTrigger')

const isDisabled = computed(() => {
  if (itemContext.disabled) return true
  if (rootContext.linear && itemContext.step > rootContext.currentStep.value + 1) return true
  return false
})

function handleClick() {
  if (isDisabled.value) return
  rootContext.goToStep(itemContext.step)
}
</script>

<template>
  <Primitive
    :id="itemContext.triggerId"
    :as="as || 'button'"
    :as-child="asChild"
    type="button"
    :aria-label="`Step ${itemContext.step}`"
    :disabled="isDisabled || undefined"
    :data-state="itemContext.state.value"
    :data-disabled="isDisabled ? '' : undefined"
    data-vize-stepper-trigger
    @click="handleClick"
  >
    <slot />
  </Primitive>
</template>
