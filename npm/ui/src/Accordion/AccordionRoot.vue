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

const internalSingle = ref<string | undefined>(
  type === 'single'
    ? (defaultValue as string | undefined)
    : undefined,
)
const internalMultiple = ref<string[]>(
  type === 'multiple'
    ? (defaultValue as string[] ?? [])
    : [],
)

const currentValue = computed<string | string[]>(() => {
  if (type === 'single') {
    return modelValue !== undefined ? (modelValue as string) : (internalSingle.value ?? '')
  } else {
    return modelValue !== undefined ? (modelValue as string[]) : internalMultiple.value
  }
})

function changeValue(itemValue: string) {
  if (disabled) return

  if (type === 'single') {
    const current = currentValue.value as string
    const next = current === itemValue && collapsible ? '' : itemValue
    internalSingle.value = next || undefined
    emit('update:modelValue', next)
  } else {
    const current = currentValue.value as string[]
    const next = current.includes(itemValue)
      ? current.filter(v => v !== itemValue)
      : [...current, itemValue]
    internalMultiple.value = next
    emit('update:modelValue', next)
  }
}

const parentRef = ref<HTMLElement>()

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
  >
    <slot />
  </Primitive>
</template>
