<script lang="ts">
export type { ToggleRootProps } from './types'
</script>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Primitive } from '../Primitive'
import type { ToggleRootProps } from './types'

const { as = 'button', asChild = false, disabled = false, defaultValue = false, modelValue } = defineProps<ToggleRootProps>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const internal = ref(defaultValue)
const pressed = computed(() => modelValue !== undefined ? modelValue : internal.value)

function toggle() {
  if (disabled) return
  const next = !pressed.value
  internal.value = next
  emit('update:modelValue', next)
}
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    type="button"
    :aria-pressed="pressed"
    :disabled="disabled || undefined"
    :data-state="pressed ? 'on' : 'off'"
    :data-disabled="disabled ? '' : undefined"
    @click="toggle"
  >
    <slot :pressed="pressed" />
  </Primitive>
</template>
