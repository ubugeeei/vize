<script lang="ts">
export type { TooltipProviderProps, TooltipProviderContext } from './types'
export { injectTooltipProviderContext, provideTooltipProviderContext } from './types'
</script>

<script setup lang="ts">
import { ref } from 'vue'
import type { TooltipProviderProps } from './types'
import { provideTooltipProviderContext } from './types'

const {
  delayDuration = 700,
  skipDelayDuration = 300,
  disableHoverableContent = false,
  disableClosingTrigger = false,
} = defineProps<TooltipProviderProps>()

const isOpenDelayed = ref(true)
let skipDelayTimer: ReturnType<typeof setTimeout> | undefined

function onOpen() {
  if (skipDelayTimer !== undefined) {
    clearTimeout(skipDelayTimer)
  }
  isOpenDelayed.value = false
}

function onClose() {
  if (skipDelayTimer !== undefined) {
    clearTimeout(skipDelayTimer)
  }
  skipDelayTimer = setTimeout(() => {
    isOpenDelayed.value = true
  }, skipDelayDuration)
}

provideTooltipProviderContext({
  delayDuration,
  skipDelayDuration,
  disableHoverableContent,
  disableClosingTrigger,
  isOpenDelayed,
  onOpen,
  onClose,
})
</script>

<template>
  <slot />
</template>
