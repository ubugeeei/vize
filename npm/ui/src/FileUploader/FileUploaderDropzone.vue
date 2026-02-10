<script lang="ts">
export type { FileUploaderDropzoneProps } from './types'
</script>

<script setup lang="ts">
import { ref } from 'vue'
import { Primitive } from '../Primitive'
import type { FileUploaderDropzoneProps } from './types'
import { injectFileUploaderRootContext } from './types'

const { as = 'div', asChild = false } = defineProps<FileUploaderDropzoneProps>()

const context = injectFileUploaderRootContext('FileUploaderDropzone')

const dragCounter = ref(0)

function onDragEnter(event: DragEvent) {
  event.preventDefault()
  event.stopPropagation()
  if (context.disabled) return

  dragCounter.value++
  if (event.dataTransfer?.items?.length) {
    context.isDragging.value = true
  }
}

function onDragOver(event: DragEvent) {
  event.preventDefault()
  event.stopPropagation()
}

function onDragLeave(event: DragEvent) {
  event.preventDefault()
  event.stopPropagation()
  if (context.disabled) return

  dragCounter.value--
  if (dragCounter.value === 0) {
    context.isDragging.value = false
  }
}

function onDrop(event: DragEvent) {
  event.preventDefault()
  event.stopPropagation()
  if (context.disabled) return

  dragCounter.value = 0
  context.isDragging.value = false

  if (event.dataTransfer?.files?.length) {
    context.addFiles(event.dataTransfer.files)
  }
}
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="presentation"
    :data-dragging="context.isDragging.value ? '' : undefined"
    :data-disabled="context.disabled ? '' : undefined"
    data-vize-file-uploader-dropzone
    @dragenter="onDragEnter"
    @dragover="onDragOver"
    @dragleave="onDragLeave"
    @drop="onDrop"
  >
    <slot :is-dragging="context.isDragging.value" />
  </Primitive>
</template>
