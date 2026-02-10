<script lang="ts">
export type { StepperContentProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { StepperContentProps } from './types'
import { injectStepperItemContext } from './types'

const { as = 'div', asChild = false, forceMount = false } = defineProps<StepperContentProps>()

const itemContext = injectStepperItemContext('StepperContent')

const isActive = computed(() => itemContext.state.value === 'active')
</script>

<template>
  <Primitive
    v-if="forceMount || isActive"
    :as="as"
    :as-child="asChild"
    role="tabpanel"
    :aria-labelledby="itemContext.triggerId"
    :data-state="itemContext.state.value"
    data-vize-stepper-content
    :style="forceMount && !isActive ? { display: 'none' } : undefined"
  >
    <slot :state="itemContext.state.value" />
  </Primitive>
</template>
