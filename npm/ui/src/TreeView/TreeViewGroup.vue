<script lang="ts">
export type { TreeViewGroupProps } from './types'
</script>

<script setup lang="ts">
import { onMounted } from 'vue'
import { Primitive } from '../Primitive'
import type { TreeViewGroupProps } from './types'
import { injectTreeViewItemContext } from './types'

const { as = 'div', asChild = false } = defineProps<TreeViewGroupProps>()

const itemContext = injectTreeViewItemContext('TreeViewGroup')

onMounted(() => {
  itemContext.isLeaf.value = false
})
</script>

<template>
  <Primitive
    v-show="itemContext.isExpanded.value"
    :as="as"
    :as-child="asChild"
    role="group"
    data-vize-tree-view-group
  >
    <slot />
  </Primitive>
</template>
