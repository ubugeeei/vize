<script lang="ts">
export type { RadioGroupIndicatorProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import { injectRadioGroupItemContext } from './types'
import type { RadioGroupIndicatorProps } from './types'

const { as = 'span', asChild = false } = defineProps<RadioGroupIndicatorProps>()

const context = injectRadioGroupItemContext('RadioGroupIndicator')
</script>

<template>
  <Primitive
    v-if="context.checked.value"
    :as="as || 'span'"
    :as-child="asChild"
    :data-state="context.checked.value ? 'checked' : 'unchecked'"
    :data-disabled="context.disabled ? '' : undefined"
    data-vize-radio-group-indicator
  >
    <slot />
  </Primitive>
</template>
