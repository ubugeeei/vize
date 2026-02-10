<script lang="ts">
export type { StepperRootProps, StepperRootContext } from './types'
export { injectStepperRootContext, provideStepperRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Primitive } from '../Primitive'
import type { StepperRootProps } from './types'
import { provideStepperRootContext } from './types'

const {
  as = 'div',
  asChild = false,
  modelValue,
  defaultValue = 1,
  orientation = 'horizontal',
  linear = false,
} = defineProps<StepperRootProps>()

const emit = defineEmits<{
  'update:modelValue': [step: number]
}>()

const internalStep = ref<number>(defaultValue)

const currentStep = computed<number>(() => {
  return modelValue !== undefined ? modelValue : internalStep.value
})

const totalSteps = ref(0)
let stepCounter = 0

function registerStep(): number {
  stepCounter += 1
  totalSteps.value = stepCounter
  return stepCounter
}

const isFirstStep = computed(() => currentStep.value <= 1)
const isLastStep = computed(() => currentStep.value >= totalSteps.value)

function goToStep(step: number) {
  if (step < 1 || step > totalSteps.value) return
  if (linear && step > currentStep.value + 1) return
  internalStep.value = step
  emit('update:modelValue', step)
}

function nextStep() {
  if (!isLastStep.value) {
    goToStep(currentStep.value + 1)
  }
}

function prevStep() {
  if (!isFirstStep.value) {
    goToStep(currentStep.value - 1)
  }
}

provideStepperRootContext({
  currentStep,
  totalSteps,
  orientation,
  linear,
  goToStep,
  nextStep,
  prevStep,
  isFirstStep,
  isLastStep,
  registerStep,
})
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="group"
    aria-label="Progress"
    :data-orientation="orientation"
    data-vize-stepper
  >
    <slot
      :current-step="currentStep"
      :total-steps="totalSteps"
      :is-first-step="isFirstStep"
      :is-last-step="isLastStep"
      :go-to-step="goToStep"
      :next-step="nextStep"
      :prev-step="prevStep"
    />
  </Primitive>
</template>
