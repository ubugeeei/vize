<script lang="ts">
export type { CheckboxIndicatorProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { injectCheckboxRootContext } from './types'
import type { CheckboxIndicatorProps } from './types'

const { as = 'span', asChild = false } = defineProps<CheckboxIndicatorProps>()

const context = injectCheckboxRootContext('CheckboxIndicator')
const isPresent = computed(() => context.state.value === true || context.state.value === 'indeterminate')
</script>

<template>
  <Primitive
    v-if="isPresent"
    :as="as || 'span'"
    :as-child="asChild"
    :data-state="context.state.value === 'indeterminate' ? 'indeterminate' : context.state.value ? 'checked' : 'unchecked'"
    :data-disabled="context.disabled ? '' : undefined"
    data-vize-checkbox-indicator
  >
    <slot />
  </Primitive>
</template>
