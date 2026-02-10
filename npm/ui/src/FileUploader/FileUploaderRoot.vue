<script lang="ts">
export type { FileUploaderRootProps, FileRejection } from './types'
export { injectFileUploaderRootContext, provideFileUploaderRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Primitive } from '../Primitive'
import type { FileUploaderRootProps, FileRejection } from './types'
import { provideFileUploaderRootContext } from './types'

const {
  as = 'div',
  asChild = false,
  modelValue,
  multiple = false,
  accept,
  maxFiles,
  maxSize,
  disabled = false,
} = defineProps<FileUploaderRootProps>()

const emit = defineEmits<{
  'update:modelValue': [files: File[]]
  'filesRejected': [rejections: FileRejection[]]
}>()

const internal = ref<File[]>([])
const files = computed(() => modelValue !== undefined ? modelValue : internal.value)
const isDragging = ref(false)
const inputRef = ref<HTMLInputElement>()

function isAcceptedType(file: File): boolean {
  if (!accept) return true

  const acceptedTypes = accept.split(',').map(t => t.trim())

  for (const type of acceptedTypes) {
    if (type.startsWith('.')) {
      if (file.name.toLowerCase().endsWith(type.toLowerCase())) return true
    } else if (type.endsWith('/*')) {
      const category = type.slice(0, type.indexOf('/'))
      if (file.type.startsWith(`${category}/`)) return true
    } else {
      if (file.type === type) return true
    }
  }

  return false
}

function addFiles(incoming: FileList | File[]) {
  if (disabled) return

  const fileArray = Array.from(incoming)
  const accepted: File[] = []
  const rejected: FileRejection[] = []

  for (const file of fileArray) {
    if (!isAcceptedType(file)) {
      rejected.push({ file, reason: 'type' })
      continue
    }

    if (maxSize !== undefined && file.size > maxSize) {
      rejected.push({ file, reason: 'size' })
      continue
    }

    accepted.push(file)
  }

  let nextFiles: File[]

  if (multiple) {
    nextFiles = [...files.value, ...accepted]
  } else {
    nextFiles = accepted.length > 0 ? [accepted[0]] : [...files.value]
  }

  if (maxFiles !== undefined && nextFiles.length > maxFiles) {
    const overflow = nextFiles.splice(maxFiles)
    for (const file of overflow) {
      rejected.push({ file, reason: 'count' })
    }
  }

  internal.value = nextFiles
  emit('update:modelValue', nextFiles)

  if (rejected.length > 0) {
    emit('filesRejected', rejected)
  }
}

function removeFile(index: number) {
  if (disabled) return

  const nextFiles = files.value.filter((_, i) => i !== index)
  internal.value = nextFiles
  emit('update:modelValue', nextFiles)
}

function clearFiles() {
  if (disabled) return

  internal.value = []
  emit('update:modelValue', [])
}

function openFilePicker() {
  if (disabled) return
  inputRef.value?.click()
}

function onInputChange(event: Event) {
  const target: HTMLInputElement = event.target
  if (target.files && target.files.length > 0) {
    addFiles(target.files)
  }
  target.value = ''
}

provideFileUploaderRootContext({
  files,
  isDragging,
  disabled,
  multiple,
  accept,
  maxFiles,
  maxSize,
  addFiles,
  removeFile,
  clearFiles,
  openFilePicker,
  inputRef,
})
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    :data-disabled="disabled ? '' : undefined"
    data-vize-file-uploader
  >
    <slot :files="files" :is-dragging="isDragging" :open-file-picker="openFilePicker" />
    <input
      :ref="(el) => { inputRef = el?.$el ?? el }"
      type="file"
      tabindex="-1"
      :multiple="multiple"
      :accept="accept"
      :disabled="disabled"
      style="display: none;"
      @change="onInputChange"
    />
  </Primitive>
</template>
