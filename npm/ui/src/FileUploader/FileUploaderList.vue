<script lang="ts">
export type { FileUploaderListProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { FileUploaderListProps } from './types'
import { injectFileUploaderRootContext } from './types'

const { as = 'div', asChild = false } = defineProps<FileUploaderListProps>()

const context = injectFileUploaderRootContext('FileUploaderList')
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="list"
    data-vize-file-uploader-list
  >
    <slot :files="context.files.value" />
  </Primitive>
</template>
