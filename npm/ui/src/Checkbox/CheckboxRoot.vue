<script lang="ts">
export type { CheckedState, CheckboxRootProps } from './types'
export { injectCheckboxRootContext, provideCheckboxRootContext } from './types'
export default { inheritAttrs: false }
</script>

<script setup lang="ts">
import { ref, computed, useAttrs } from 'vue'
import { Primitive } from '../Primitive'
import { useFormControl } from '../shared'
import type { CheckboxRootProps, CheckedState } from './types'
import { provideCheckboxRootContext } from './types'

const { as = 'button', asChild = false, disabled = false, required = false, name, value = 'on', defaultValue = false, modelValue } = defineProps<CheckboxRootProps>()
const emit = defineEmits<{
  'update:modelValue': [value: CheckedState]
}>()

const initState: CheckedState = defaultValue ?? false
const internal = ref<CheckedState>(initState)
const state = computed<CheckedState>(() => modelValue !== undefined ? modelValue : internal.value)

function toggle() {
  if (disabled) return
  const next = state.value === 'indeterminate' ? true : !state.value
  internal.value = next
  emit('update:modelValue', next)
}

const dataState = computed(() => {
  if (state.value === 'indeterminate') return 'indeterminate'
  return state.value ? 'checked' : 'unchecked'
})

const { BubbleInput } = useFormControl(() => ({
  type: 'checkbox',
  name,
  value,
  checked: state.value === true,
  disabled,
  required,
}))

const rootAttrs = useAttrs()
function getAriaLabel(): string | undefined {
  const v = rootAttrs['aria-label']
  if (typeof v === 'string') return v
  return undefined
}

provideCheckboxRootContext({ state, disabled })
</script>

<template>
  <Primitive
    :as="as || 'button'"
    :aria-label="getAriaLabel()"
    :as-child="asChild"
    type="button"
    role="checkbox"
    :aria-checked="state === 'indeterminate' ? 'mixed' : state"
    :aria-required="required || undefined"
    :disabled="disabled || undefined"
    :data-state="dataState"
    :data-disabled="disabled ? '' : undefined"
    data-vize-checkbox
    @click="toggle"
  >
    <slot :checked="state" />
  </Primitive>
  <BubbleInput />
</template>
