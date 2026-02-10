<script lang="ts">
export type { OTPSeparatorProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { OTPSeparatorProps } from './types'

const { as = 'span', asChild = false } = defineProps<OTPSeparatorProps>()
</script>

<template>
  <Primitive
    :as="as || 'span'"
    :as-child="asChild"
    role="separator"
    data-vize-otp-separator
  >
    <slot />
  </Primitive>
</template>
