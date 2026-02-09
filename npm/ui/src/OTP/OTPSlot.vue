<script lang="ts">
export type { OTPSlotProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { OTPSlotProps } from './types'
import { injectOTPRootContext } from './types'

const { as = 'div', asChild = false, index } = defineProps<OTPSlotProps>()

const context = injectOTPRootContext('OTPSlot')

const char = computed(() => {
  const idx = index
  const rawChar = context.value.value[idx] ?? ''
  if (!rawChar) return ''

  if (context.mask === true) return '\u2022'
  if (typeof context.mask === 'string' && context.mask) return context.mask
  return rawChar
})

const rawChar = computed(() => {
  const idx = index
  return context.value.value[idx] ?? ''
})
const hasFakeCaret = computed(() => context.isInputFocused.value && context.focusedIndex.value === index && !rawChar.value)
const isActive = computed(() => context.isInputFocused.value && context.focusedIndex.value === index)
const isFilled = computed(() => !!rawChar.value)
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    :data-filled="isFilled ? '' : undefined"
    :data-focused="isActive ? '' : undefined"
    :data-active="isActive ? '' : undefined"
    data-vize-otp-slot
  >
    <slot :char="char" :has-fake-caret="hasFakeCaret" :is-active="isActive" />
    <template v-if="!$slots.default">
      {{ char }}
    </template>
  </Primitive>
</template>
