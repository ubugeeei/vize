<script lang="ts">
export type { AccordionItemProps, AccordionItemContext } from './types'
export { injectAccordionItemContext, provideAccordionItemContext } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { useId } from '../shared'
import { injectAccordionRootContext } from './types'
import type { AccordionItemProps } from './types'
import { provideAccordionItemContext } from './types'

const { as = 'div', asChild = false, value, disabled = false } = defineProps<AccordionItemProps>()

const rootContext = injectAccordionRootContext('AccordionItem')

const triggerId = useId()
const contentId = useId()

const open = computed(() => {
  if (rootContext.type === 'single') {
    return rootContext.modelValue.value === value
  } else {
    const arr = Array.isArray(rootContext.modelValue.value) ? rootContext.modelValue.value : []
    return arr.includes(value)
  }
})

const isDisabled = computed(() => disabled || rootContext.disabled)

provideAccordionItemContext({
  value,
  triggerId,
  contentId,
  open,
  disabled: isDisabled,
})
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    :data-state="open ? 'open' : 'closed'"
    :data-disabled="isDisabled ? '' : undefined"
    :data-orientation="rootContext.orientation"
    data-vize-accordion-item
  >
    <slot :open="open" />
  </Primitive>
</template>
