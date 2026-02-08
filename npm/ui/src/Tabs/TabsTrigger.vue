<script lang="ts">
export type { TabsTriggerProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { useId } from '../shared'
import { injectTabsRootContext } from './types'
import type { TabsTriggerProps } from './types'

const { as = 'button', asChild = false, value, disabled = false } = defineProps<TabsTriggerProps>()

const rootContext = injectTabsRootContext('TabsTrigger')

const triggerId = useId()
const panelId = useId()

const isSelected = computed(() => rootContext.modelValue.value === value)

function handleClick() {
  if (disabled) return
  rootContext.changeValue(value)
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === ' ' || event.key === 'Enter') {
    if (rootContext.activationMode === 'manual') {
      event.preventDefault()
      rootContext.changeValue(value)
    }
  }
}

function handleFocus() {
  if (rootContext.activationMode === 'automatic' && !disabled) {
    rootContext.changeValue(value)
  }
}
</script>

<template>
  <Primitive
    :id="triggerId"
    :as="as"
    :as-child="asChild"
    type="button"
    role="tab"
    :aria-selected="isSelected"
    :aria-controls="`${panelId}-panel-${value}`"
    :tabindex="isSelected ? 0 : -1"
    :disabled="disabled || undefined"
    :data-state="isSelected ? 'active' : 'inactive'"
    :data-disabled="disabled ? '' : undefined"
    :data-orientation="rootContext.orientation"
    :data-value="value"
    data-vize-collection-item
    @click="handleClick"
    @keydown="handleKeydown"
    @focus="handleFocus"
  >
    <slot />
  </Primitive>
</template>
