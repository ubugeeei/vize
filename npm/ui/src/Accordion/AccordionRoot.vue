<script lang="ts">
export type { AccordionRootProps, AccordionRootContext } from './types'
export { injectAccordionRootContext, provideAccordionRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed, toRef } from 'vue'
import { Primitive } from '../Primitive'
import { useDirection } from '../shared'
import type { AccordionRootProps } from './types'
import { provideAccordionRootContext } from './types'

const {
  as = 'div',
  asChild = false,
  type = 'single',
  disabled = false,
  collapsible = false,
  orientation = 'vertical',
  dir: dirProp,
  defaultValue,
  modelValue,
} = defineProps<AccordionRootProps>()

const emit = defineEmits<{
  'update:modelValue': [value: string | string[]]
}>()

const direction = useDirection(computed(() => dirProp))

// NOTE: Avoid TypeScript `as` casts in this file.
// The native Vue compiler confuses the `as` keyword with the `as` prop.

function toSingleDefault(v: AccordionRootProps['defaultValue']): string | undefined {
  return v != null ? String(v) : undefined
}

function toMultipleDefault(v: AccordionRootProps['defaultValue']): string[] {
  if (v == null) return []
  return Array.isArray(v) ? v : [String(v)]
}

const singleInit: string | undefined = type === 'single' ? toSingleDefault(defaultValue) : undefined
const internalSingle = ref(singleInit)

const multipleInit: string[] = type === 'multiple' ? toMultipleDefault(defaultValue) : []
const internalMultiple = ref(multipleInit)

const currentValue = computed(() => {
  if (type === 'single') {
    if (modelValue !== undefined) {
      return String(modelValue)
    }
    return internalSingle.value ?? ''
  } else {
    if (modelValue !== undefined) {
      return Array.isArray(modelValue) ? modelValue : []
    }
    return internalMultiple.value
  }
})

function changeValue(itemValue: string) {
  if (disabled) return

  if (type === 'single') {
    const current = String(currentValue.value)
    const next = current === itemValue && collapsible ? '' : itemValue
    internalSingle.value = next || undefined
    emit('update:modelValue', next)
  } else {
    const current = Array.isArray(currentValue.value) ? currentValue.value : []
    const next = current.includes(itemValue)
      ? current.filter(v => v !== itemValue)
      : [...current, itemValue]
    internalMultiple.value = next
    emit('update:modelValue', next)
  }
}

const parentElement: HTMLElement | undefined = undefined
const parentRef = ref(parentElement)

provideAccordionRootContext({
  type,
  modelValue: currentValue,
  disabled,
  collapsible,
  orientation,
  dir: direction,
  changeValue,
  parentRef,
})
</script>

<template>
  <Primitive
    :ref="(el) => { parentRef = el?.$el ?? el }"
    :as="as"
    :as-child="asChild"
    :data-orientation="orientation"
    data-vize-accordion
  >
    <slot />
  </Primitive>
</template>
