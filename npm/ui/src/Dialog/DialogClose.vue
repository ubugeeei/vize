<script lang="ts">
export type { DialogCloseProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import { injectDialogRootContext } from './types'
import type { DialogCloseProps } from './types'

const { as = 'button', asChild = false } = defineProps<DialogCloseProps>()

const context = injectDialogRootContext('DialogClose')

function handleClick() {
  context.onOpenChange(false)
}
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    type="button"
    @click="handleClick"
  >
    <slot />
  </Primitive>
</template>
