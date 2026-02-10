<script lang="ts">
export type { FileUploaderItemProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { FileUploaderItemProps } from './types'
import { injectFileUploaderRootContext } from './types'

const { as = 'div', asChild = false, index } = defineProps<FileUploaderItemProps>()

const context = injectFileUploaderRootContext('FileUploaderItem')

const file = computed(() => context.files.value[index])
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="listitem"
    :data-index="index"
    data-vize-file-uploader-item
  >
    <slot :file="file" :index="index" />
  </Primitive>
</template>
