<script lang="ts">
export type { OTPRootProps } from './types'
export { injectOTPRootContext, provideOTPRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Primitive } from '../Primitive'
import { kbd } from '../shared'
import type { OTPRootProps } from './types'
import { provideOTPRootContext } from './types'

const {
  as = 'div',
  asChild = false,
  modelValue,
  defaultValue = '',
  length = 6,
  disabled = false,
  inputMode = 'numeric',
  mask = false,
  autoSubmit = false,
} = defineProps<OTPRootProps>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'complete': [value: string]
}>()

const initValue: string = defaultValue ?? ''
const internal = ref(initValue)
const value = computed(() => modelValue !== undefined ? modelValue : internal.value)

const focusedIndex = ref(-1)
const isInputFocused = ref(false)
const hiddenInputRef = ref<HTMLInputElement>()

const isFilled = computed(() => value.value.length >= length)

function setValue(newValue: string) {
  const clamped = newValue.slice(0, length)
  internal.value = clamped
  emit('update:modelValue', clamped)

  if (autoSubmit && clamped.length >= length) {
    emit('complete', clamped)
  }
}

function setChar(index: number, char: string) {
  const current = value.value.padEnd(length, '')
  const chars = current.split('')
  chars[index] = char
  const newValue = chars.join('').replace(/\s+$/, '')
  setValue(newValue)

  if (index < length - 1) {
    focusedIndex.value = index + 1
  }
}

function deleteChar(index: number) {
  const current = value.value.padEnd(length, '')
  const chars = current.split('')
  chars[index] = ''
  const newValue = chars.join('').replace(/\s+$/, '')
  setValue(newValue)
}

function focusIndex(index: number) {
  const clamped = Math.max(0, Math.min(index, length - 1))
  focusedIndex.value = clamped
  hiddenInputRef.value?.focus()
}

function isValidChar(char: string): boolean {
  if (inputMode === 'numeric') {
    return /^\d$/.test(char)
  }
  return /^[\w]$/.test(char)
}

function onHiddenInputInput(event: Event) {
  event.preventDefault()
  const target: HTMLInputElement = event.target
  const inputValue = target.value

  if (inputValue.length > 1) {
    // Paste or multi-character input
    handlePaste(inputValue)
  } else if (inputValue.length === 1 && isValidChar(inputValue)) {
    setChar(focusedIndex.value, inputValue)
  }

  target.value = ''
}

function handlePaste(text: string) {
  const chars = text.split('').filter(isValidChar).slice(0, length - focusedIndex.value)
  if (chars.length === 0) return

  const current = value.value.padEnd(length, '')
  const arr = current.split('')
  for (let i = 0; i < chars.length; i++) {
    arr[focusedIndex.value + i] = chars[i]
  }
  const newValue = arr.join('').replace(/\s+$/, '')
  setValue(newValue)

  const nextIndex = Math.min(focusedIndex.value + chars.length, length - 1)
  focusedIndex.value = nextIndex
}

function onHiddenInputKeydown(event: KeyboardEvent) {
  if (event.key === kbd.ARROW_LEFT) {
    event.preventDefault()
    if (focusedIndex.value > 0) {
      focusedIndex.value--
    }
  } else if (event.key === kbd.ARROW_RIGHT) {
    event.preventDefault()
    if (focusedIndex.value < length - 1) {
      focusedIndex.value++
    }
  } else if (event.key === 'Backspace') {
    event.preventDefault()
    const current = value.value
    if (focusedIndex.value < current.length) {
      deleteChar(focusedIndex.value)
    }
    if (focusedIndex.value > 0) {
      focusedIndex.value--
    }
  } else if (event.key === 'Delete') {
    event.preventDefault()
    deleteChar(focusedIndex.value)
  } else if (event.key === kbd.HOME) {
    event.preventDefault()
    focusedIndex.value = 0
  } else if (event.key === kbd.END) {
    event.preventDefault()
    focusedIndex.value = length - 1
  }
}

function onHiddenInputFocus() {
  isInputFocused.value = true
  if (focusedIndex.value < 0) {
    focusedIndex.value = Math.min(value.value.length, length - 1)
  }
}

function onHiddenInputBlur() {
  isInputFocused.value = false
  focusedIndex.value = -1
}

function onHiddenInputPaste(event: ClipboardEvent) {
  event.preventDefault()
  const text = event.clipboardData?.getData('text/plain')
  if (text) {
    handlePaste(text)
  }
}

function onRootClick() {
  if (disabled) return
  hiddenInputRef.value?.focus()
  if (focusedIndex.value < 0) {
    focusedIndex.value = Math.min(value.value.length, length - 1)
  }
}

provideOTPRootContext({
  value,
  length,
  disabled,
  inputMode,
  mask,
  focusedIndex,
  setChar,
  deleteChar,
  focusIndex,
  isFilled,
  isInputFocused,
})
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="group"
    aria-label="One-time password"
    :data-complete="isFilled ? '' : undefined"
    :data-disabled="disabled ? '' : undefined"
    data-vize-otp
    style="position: relative;"
    @click="onRootClick"
  >
    <slot />
    <input
      :ref="(el) => { hiddenInputRef = el?.$el ?? el }"
      :inputmode="inputMode === 'numeric' ? 'numeric' : 'text'"
      :autocomplete="'one-time-code'"
      :disabled="disabled"
      :value="''"
      aria-hidden="true"
      tabindex="0"
      style="position: absolute; inset: 0; width: 100%; height: 100%; opacity: 0; pointer-events: none; caret-color: transparent;"
      @input="onHiddenInputInput"
      @keydown="onHiddenInputKeydown"
      @focus="onHiddenInputFocus"
      @blur="onHiddenInputBlur"
      @paste="onHiddenInputPaste"
    />
  </Primitive>
</template>
