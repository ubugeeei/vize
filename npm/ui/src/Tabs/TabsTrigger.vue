<script lang="ts">
export type { TabsTriggerProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { injectTabsRootContext } from './types'
import type { TabsTriggerProps } from './types'

const { as = 'button', asChild = false, value, disabled = false } = defineProps<TabsTriggerProps>()

const rootContext = injectTabsRootContext('TabsTrigger')

const triggerId = rootContext.baseId.concat('-trigger-', value)
const panelId = rootContext.baseId.concat('-panel-', value)

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
    :as="as || 'button'"
    :as-child="asChild"
    type="button"
    role="tab"
    :aria-selected="isSelected"
    :aria-controls="panelId"
    :tabindex="isSelected ? 0 : -1"
    :disabled="disabled || undefined"
    :data-state="isSelected ? 'active' : 'inactive'"
    :data-disabled="disabled ? '' : undefined"
    :data-orientation="rootContext.orientation"
    :data-value="value"
    data-vize-collection-item
    data-vize-tabs-trigger
    @click="handleClick"
    @keydown="handleKeydown"
    @focus="handleFocus"
  >
    <slot />
  </Primitive>
</template>
