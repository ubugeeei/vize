<script lang="ts">
export type { CollapsibleTriggerProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import { injectCollapsibleRootContext } from './types'
import type { CollapsibleTriggerProps } from './types'

const { as = 'button', asChild = false } = defineProps<CollapsibleTriggerProps>()

const context = injectCollapsibleRootContext('CollapsibleTrigger')
</script>

<template>
  <Primitive
    :as="as || 'button'"
    :as-child="asChild"
    type="button"
    :aria-controls="context.contentId"
    :aria-expanded="context.open.value"
    :disabled="context.disabled || undefined"
    :data-state="context.open.value ? 'open' : 'closed'"
    :data-disabled="context.disabled ? '' : undefined"
    data-vize-collapsible-trigger
    @click="context.toggle()"
  >
    <slot />
  </Primitive>
</template>
