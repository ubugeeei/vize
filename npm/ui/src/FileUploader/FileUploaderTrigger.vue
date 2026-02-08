<script lang="ts">
export type { FileUploaderTriggerProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { FileUploaderTriggerProps } from './types'
import { injectFileUploaderRootContext } from './types'

const { as = 'button', asChild = false } = defineProps<FileUploaderTriggerProps>()

const context = injectFileUploaderRootContext('FileUploaderTrigger')

function onClick() {
  context.openFilePicker()
}
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    type="button"
    :disabled="context.disabled || undefined"
    :data-disabled="context.disabled ? '' : undefined"
    @click="onClick"
  >
    <slot />
  </Primitive>
</template>
