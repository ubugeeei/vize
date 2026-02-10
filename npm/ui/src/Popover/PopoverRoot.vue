<script lang="ts">
export type { PopoverRootProps, PopoverRootContext } from './types'
export { injectPopoverRootContext, providePopoverRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useId } from '../shared'
import type { PopoverRootProps } from './types'
import { providePopoverRootContext } from './types'

const { defaultValue = false, modelValue, modal = false } = defineProps<PopoverRootProps>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const contentId = useId()
const triggerRef = ref<HTMLElement>()

const internal = ref(defaultValue)
const open = computed(() => modelValue !== undefined ? modelValue : internal.value)

function onOpenChange(value: boolean) {
  internal.value = value
  emit('update:modelValue', value)
}

providePopoverRootContext({
  open,
  onOpenChange,
  triggerRef,
  contentId,
  modal,
})
</script>

<template>
  <slot :open="open" />
</template>
