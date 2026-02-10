<script lang="ts">
export type { PopoverCloseProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import { injectPopoverRootContext } from './types'
import type { PopoverCloseProps } from './types'

const { as = 'button', asChild = false } = defineProps<PopoverCloseProps>()

const context = injectPopoverRootContext('PopoverClose')

function handleClick() {
  context.onOpenChange(false)
}
</script>

<template>
  <Primitive
    :as="as || 'button'"
    :as-child="asChild"
    type="button"
    data-vize-popover-close
    @click="handleClick"
  >
    <slot />
  </Primitive>
</template>
