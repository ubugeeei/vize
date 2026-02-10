<script lang="ts">
export type { RadioGroupItemProps } from './types'
export { injectRadioGroupItemContext, provideRadioGroupItemContext } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { useFormControl } from '../shared'
import { injectRadioGroupRootContext } from './types'
import type { RadioGroupItemProps } from './types'
import { provideRadioGroupItemContext } from './types'

const { as = 'button', asChild = false, value, disabled = false } = defineProps<RadioGroupItemProps>()

const context = injectRadioGroupRootContext('RadioGroupItem')
const isDisabled = computed(() => disabled || context.disabled)
const checked = computed(() => context.modelValue.value === value)

function handleClick() {
  if (isDisabled.value) return
  context.updateValue(value)
}

const dataState = computed(() => checked.value ? 'checked' : 'unchecked')

const { BubbleInput } = useFormControl(() => ({
  type: 'radio',
  name: context.name,
  value,
  checked: checked.value,
  disabled: isDisabled.value,
  required: context.required,
}))

provideRadioGroupItemContext({
  checked,
  disabled: isDisabled.value,
})
</script>

<template>
  <Primitive
    :as="as || 'button'"
    :as-child="asChild"
    type="button"
    role="radio"
    :aria-checked="checked"
    :disabled="isDisabled || undefined"
    :data-state="dataState"
    :data-disabled="isDisabled ? '' : undefined"
    data-vize-collection-item
    data-vize-radio-group-item
    @click="handleClick"
  >
    <slot />
  </Primitive>
  <BubbleInput />
</template>
