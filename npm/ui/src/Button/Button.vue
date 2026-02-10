<script lang="ts">
export type { ButtonProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { ButtonProps } from './types'

const { as = 'button', asChild = false, disabled = false } = defineProps<ButtonProps>()

const isNativeButton = computed(() => as === 'button')
</script>

<template>
  <Primitive
    :as="as || 'button'"
    :as-child="asChild"
    :type="isNativeButton ? 'button' : undefined"
    :role="!isNativeButton ? 'button' : undefined"
    :disabled="disabled || undefined"
    :aria-disabled="!isNativeButton && disabled ? true : undefined"
    :tabindex="!isNativeButton && !disabled ? 0 : undefined"
    :data-disabled="disabled ? '' : undefined"
    data-vize-button
  >
    <slot />
  </Primitive>
</template>
