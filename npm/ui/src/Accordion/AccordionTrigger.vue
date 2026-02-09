<script lang="ts">
export type { AccordionTriggerProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import { useArrowNavigation } from '../shared'
import { injectAccordionRootContext } from './types'
import { injectAccordionItemContext } from './types'
import type { AccordionTriggerProps } from './types'

const { as = 'button', asChild = false } = defineProps<AccordionTriggerProps>()

const rootContext = injectAccordionRootContext('AccordionTrigger')
const itemContext = injectAccordionItemContext('AccordionTrigger')

function handleClick() {
  if (itemContext.disabled.value) return
  rootContext.changeValue(itemContext.value)
}

function handleKeydown(event: KeyboardEvent) {
  const target = useArrowNavigation(
    event,
    event.currentTarget instanceof HTMLElement ? event.currentTarget : null,
    rootContext.parentRef.value,
    {
      orientation: rootContext.orientation,
      loop: true,
      dir: rootContext.dir.value,
    },
  )

  if (target) {
    event.preventDefault()
    target.focus()
  }
}
</script>

<template>
  <Primitive
    :id="itemContext.triggerId"
    :as="as"
    :as-child="asChild"
    type="button"
    :aria-expanded="itemContext.open.value"
    :aria-controls="itemContext.contentId"
    :aria-disabled="itemContext.disabled.value || undefined"
    :disabled="itemContext.disabled.value || undefined"
    :data-state="itemContext.open.value ? 'open' : 'closed'"
    :data-disabled="itemContext.disabled.value ? '' : undefined"
    :data-orientation="rootContext.orientation"
    data-vize-collection-item
    data-vize-accordion-trigger
    @click="handleClick"
    @keydown="handleKeydown"
  >
    <slot />
  </Primitive>
</template>
