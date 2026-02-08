<script lang="ts">
export type { DialogRootProps, DialogRootContext } from './types'
export { injectDialogRootContext, provideDialogRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useId } from '../shared'
import type { DialogRootProps } from './types'
import { provideDialogRootContext } from './types'

const { defaultValue = false, modelValue } = defineProps<DialogRootProps>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const titleId = useId()
const descriptionId = useId()
const contentId = useId()
const triggerRef = ref<HTMLElement>()

const internal = ref(defaultValue)
const open = computed(() => modelValue !== undefined ? modelValue : internal.value)

function onOpenChange(value: boolean) {
  internal.value = value
  emit('update:modelValue', value)
}

provideDialogRootContext({
  open,
  onOpenChange,
  titleId,
  descriptionId,
  triggerRef,
  contentId,
})
</script>

<template>
  <slot :open="open" />
</template>
