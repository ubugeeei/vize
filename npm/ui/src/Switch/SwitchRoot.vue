<script lang="ts">
export type { SwitchRootProps } from './types'
export { injectSwitchRootContext, provideSwitchRootContext } from './types'
export default { inheritAttrs: false }
</script>

<script setup lang="ts">
import { ref, computed, useAttrs } from 'vue'
import { Primitive } from '../Primitive'
import { useFormControl } from '../shared'
import type { SwitchRootProps } from './types'
import { provideSwitchRootContext } from './types'

const { as = 'button', asChild = false, disabled = false, required = false, name, value = 'on', defaultValue = false, modelValue } = defineProps<SwitchRootProps>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const initChecked: boolean = defaultValue ?? false
const internal = ref(initChecked)
const checked = computed(() => modelValue !== undefined ? modelValue : internal.value)

function toggle() {
  if (disabled) return
  const next = !checked.value
  internal.value = next
  emit('update:modelValue', next)
}

const dataState = computed(() => checked.value ? 'checked' : 'unchecked')

const { BubbleInput } = useFormControl(() => ({
  type: 'checkbox',
  name,
  value,
  checked: checked.value,
  disabled,
  required,
}))

const rootAttrs = useAttrs()
function getAriaLabel(): string | undefined {
  const v = rootAttrs['aria-label']
  if (typeof v === 'string') return v
  return undefined
}

provideSwitchRootContext({ checked, disabled })
</script>

<template>
  <Primitive
    :as="as || 'button'"
    :aria-label="getAriaLabel()"
    :as-child="asChild"
    type="button"
    role="switch"
    :aria-checked="checked"
    :aria-required="required || undefined"
    :disabled="disabled || undefined"
    :data-state="dataState"
    :data-disabled="disabled ? '' : undefined"
    data-vize-switch
    @click="toggle"
  >
    <slot :checked="checked" />
  </Primitive>
  <BubbleInput />
</template>
