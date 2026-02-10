<script lang="ts">
export type { FileUploaderItemDeleteProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { FileUploaderItemDeleteProps } from './types'
import { injectFileUploaderRootContext } from './types'

const { as = 'button', asChild = false, index } = defineProps<FileUploaderItemDeleteProps>()

const context = injectFileUploaderRootContext('FileUploaderItemDelete')

function onClick() {
  context.removeFile(index)
}
</script>

<template>
  <Primitive
    :as="as || 'button'"
    :as-child="asChild"
    type="button"
    aria-label="Remove file"
    :disabled="context.disabled || undefined"
    :data-disabled="context.disabled ? '' : undefined"
    data-vize-file-uploader-item-delete
    @click="onClick"
  >
    <slot />
  </Primitive>
</template>
