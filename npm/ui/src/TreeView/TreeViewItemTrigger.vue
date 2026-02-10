<script lang="ts">
export type { TreeViewItemTriggerProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { TreeViewItemTriggerProps } from './types'
import { injectTreeViewRootContext, injectTreeViewItemContext } from './types'

const { as = 'div', asChild = false } = defineProps<TreeViewItemTriggerProps>()

const rootContext = injectTreeViewRootContext('TreeViewItemTrigger')
const itemContext = injectTreeViewItemContext('TreeViewItemTrigger')

function handleClick() {
  if (itemContext.disabled) return
  rootContext.selectNode(itemContext.value)
  if (!itemContext.isLeaf.value) {
    rootContext.toggleExpanded(itemContext.value)
  }
}
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    :data-state="itemContext.isSelected.value ? 'selected' : 'unselected'"
    :data-disabled="itemContext.disabled ? '' : undefined"
    data-vize-tree-view-item-trigger
    @click="handleClick"
  >
    <slot />
  </Primitive>
</template>
