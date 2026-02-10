<script lang="ts">
export type { TooltipRootProps, TooltipRootContext } from './types'
export { injectTooltipRootContext, provideTooltipRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed, onBeforeUnmount } from 'vue'
import { useId } from '../shared'
import { injectTooltipProviderContext } from './types'
import type { TooltipRootProps } from './types'
import { provideTooltipRootContext } from './types'

const { defaultValue = false, modelValue, delayDuration } = defineProps<TooltipRootProps>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const provider = injectTooltipProviderContext('TooltipRoot')

const contentId = useId()
const triggerRef = ref<HTMLElement>()

const internal = ref(defaultValue)
const open = computed(() => modelValue !== undefined ? modelValue : internal.value)

let openTimer: ReturnType<typeof setTimeout> | undefined

const resolvedDelayDuration = computed(() =>
  delayDuration !== undefined ? delayDuration : provider.delayDuration,
)

function onOpenChange(value: boolean) {
  if (openTimer !== undefined) {
    clearTimeout(openTimer)
    openTimer = undefined
  }

  if (value) {
    const delay = provider.isOpenDelayed.value ? resolvedDelayDuration.value : 0
    if (delay > 0) {
      openTimer = setTimeout(() => {
        internal.value = true
        emit('update:modelValue', true)
        provider.onOpen()
      }, delay)
    } else {
      internal.value = true
      emit('update:modelValue', true)
      provider.onOpen()
    }
  } else {
    internal.value = false
    emit('update:modelValue', false)
    provider.onClose()
  }
}

onBeforeUnmount(() => {
  if (openTimer !== undefined) {
    clearTimeout(openTimer)
  }
})

provideTooltipRootContext({
  open,
  onOpenChange,
  triggerRef,
  contentId,
  isDisabled: false,
})
</script>

<template>
  <slot :open="open" />
</template>
