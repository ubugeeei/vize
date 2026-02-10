<script lang="ts">
export type { AccordionHeaderProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import { injectAccordionRootContext } from './types'
import { injectAccordionItemContext } from './types'
import type { AccordionHeaderProps } from './types'

const { as = 'h3', asChild = false } = defineProps<AccordionHeaderProps>()

const rootContext = injectAccordionRootContext('AccordionHeader')
const itemContext = injectAccordionItemContext('AccordionHeader')
</script>

<template>
  <Primitive
    :as="as || 'h3'"
    :as-child="asChild"
    :data-state="itemContext.open.value ? 'open' : 'closed'"
    :data-disabled="itemContext.disabled.value ? '' : undefined"
    :data-orientation="rootContext.orientation"
    data-vize-accordion-header
  >
    <slot />
  </Primitive>
</template>
