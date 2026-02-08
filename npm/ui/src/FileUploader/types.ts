import type { ComputedRef, Ref } from 'vue'
import type { PrimitiveProps } from '../Primitive'
import { createContext } from '../shared'

export interface FileUploaderRootProps extends PrimitiveProps {
  modelValue?: File[]
  multiple?: boolean
  accept?: string
  maxFiles?: number
  maxSize?: number
  disabled?: boolean
}

export interface FileRejection {
  file: File
  reason: 'type' | 'size' | 'count'
}

export interface FileUploaderRootContext {
  files: ComputedRef<File[]>
  isDragging: Ref<boolean>
  disabled: boolean
  multiple: boolean
  accept: string | undefined
  maxFiles: number | undefined
  maxSize: number | undefined
  addFiles: (files: FileList | File[]) => void
  removeFile: (index: number) => void
  clearFiles: () => void
  openFilePicker: () => void
  inputRef: Ref<HTMLInputElement | undefined>
}

const _fileUploaderRootContext = createContext<FileUploaderRootContext>('FileUploaderRoot')
export const injectFileUploaderRootContext = _fileUploaderRootContext[0]
export const provideFileUploaderRootContext = _fileUploaderRootContext[1]

export interface FileUploaderDropzoneProps extends PrimitiveProps {}

export interface FileUploaderTriggerProps extends PrimitiveProps {}

export interface FileUploaderListProps extends PrimitiveProps {}

export interface FileUploaderItemProps extends PrimitiveProps {
  index: number
}

export interface FileUploaderItemDeleteProps extends PrimitiveProps {
  index: number
}
