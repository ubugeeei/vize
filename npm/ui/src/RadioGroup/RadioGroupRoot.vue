<script lang="ts">
export type { RadioGroupRootProps } from './types'
export { injectRadioGroupRootContext, provideRadioGroupRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Primitive } from '../Primitive'
import { useArrowNavigation, useDirection } from '../shared'
import type { RadioGroupRootProps } from './types'
import { provideRadioGroupRootContext } from './types'

const { as = 'div', asChild = false, disabled = false, required = false, name, orientation = 'vertical', loop = true, dir: dirProp, defaultValue, modelValue } = defineProps<RadioGroupRootProps>()
const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const direction = useDirection(computed(() => dirProp))

const internal = ref(defaultValue)
const currentValue = computed(() => modelValue !== undefined ? modelValue : internal.value)

function updateValue(value: string) {
  if (disabled) return
  internal.value = value
  emit('update:modelValue', value)
}

function handleKeydown(event: KeyboardEvent) {
  const target: HTMLElement = event.target
  const rootEl: HTMLElement = event.currentTarget
  const nextElement = useArrowNavigation(event, target, rootEl, {
    orientation,
    loop,
    dir: direction.value,
  })
  if (nextElement) {
    event.preventDefault()
    nextElement.focus()
    nextElement.click()
  }
}

provideRadioGroupRootContext({
  modelValue: currentValue,
  disabled,
  required,
  name,
  orientation,
  loop,
  dir: direction.value,
  updateValue,
})
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="radiogroup"
    :aria-required="required || undefined"
    :aria-orientation="orientation"
    :data-orientation="orientation"
    :data-disabled="disabled ? '' : undefined"
    data-vize-radio-group
    :dir="direction"
    @keydown="handleKeydown"
  >
    <slot />
  </Primitive>
</template>
