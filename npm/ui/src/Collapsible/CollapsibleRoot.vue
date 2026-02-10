<script lang="ts">
export type { CollapsibleRootProps } from './types'
export { injectCollapsibleRootContext, provideCollapsibleRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Primitive } from '../Primitive'
import { useId } from '../shared'
import type { CollapsibleRootProps } from './types'
import { provideCollapsibleRootContext } from './types'

const { as = 'div', asChild = false, disabled = false, defaultValue = false, modelValue } = defineProps<CollapsibleRootProps>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const contentId = useId()
const internal = ref(defaultValue)
const open = computed(() => modelValue !== undefined ? modelValue : internal.value)

function toggle() {
  if (disabled) return
  const next = !open.value
  internal.value = next
  emit('update:modelValue', next)
}

provideCollapsibleRootContext({ contentId, open, disabled, toggle })
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    :data-state="open ? 'open' : 'closed'"
    :data-disabled="disabled ? '' : undefined"
    data-vize-collapsible
  >
    <slot :open="open" />
  </Primitive>
</template>
