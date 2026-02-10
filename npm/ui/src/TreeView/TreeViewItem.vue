<script lang="ts">
export type { TreeViewItemProps, TreeViewItemContext } from './types'
export { injectTreeViewItemContext, provideTreeViewItemContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Primitive } from '../Primitive'
import type { TreeViewItemProps } from './types'
import { injectTreeViewRootContext, injectTreeViewItemContext, provideTreeViewItemContext } from './types'

const {
  as = 'div',
  asChild = false,
  value,
  disabled = false,
} = defineProps<TreeViewItemProps>()

const rootContext = injectTreeViewRootContext('TreeViewItem')

let parentItemContext: import('./types').TreeViewItemContext | undefined
try {
  parentItemContext = injectTreeViewItemContext()
} catch {
  parentItemContext = undefined
}

const level = parentItemContext ? parentItemContext.level + 1 : 1

const isDisabled = disabled || rootContext.disabled

rootContext.registerItem(value)

const isExpanded = computed(() => rootContext.isExpanded(value))
const isSelected = computed(() => rootContext.isSelected(value))
const isFocused = computed(() => {
  // If no item has been focused yet, make the first registered item focusable
  if (rootContext.focusedId.value === undefined) {
    return rootContext.firstItemId.value === value
  }
  return rootContext.focusedId.value === value
})

const isLeaf = ref(true)

provideTreeViewItemContext({
  value,
  level,
  disabled: isDisabled,
  isLeaf,
  isExpanded,
  isSelected,
})
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="treeitem"
    :aria-selected="isSelected"
    :aria-expanded="!isLeaf ? isExpanded : undefined"
    :aria-level="level"
    :aria-disabled="isDisabled || undefined"
    :tabindex="isFocused ? 0 : -1"
    :data-state="isSelected ? 'selected' : 'unselected'"
    :data-expanded="isExpanded ? '' : undefined"
    :data-disabled="isDisabled ? '' : undefined"
    :data-value="value"
    data-vize-tree-view-item
    @focus="rootContext.setFocused(value)"
  >
    <slot
      :expanded="isExpanded"
      :selected="isSelected"
      :level="level"
      :leaf="isLeaf"
    />
  </Primitive>
</template>
