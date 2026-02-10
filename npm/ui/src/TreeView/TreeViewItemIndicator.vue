<script lang="ts">
export type { TreeViewItemIndicatorProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { TreeViewItemIndicatorProps } from './types'
import { injectTreeViewItemContext } from './types'

const { as = 'span', asChild = false } = defineProps<TreeViewItemIndicatorProps>()

const itemContext = injectTreeViewItemContext('TreeViewItemIndicator')

const shouldRender = computed(() => !itemContext.isLeaf.value)
</script>

<template>
  <Primitive
    v-if="shouldRender"
    :as="as || 'span'"
    :as-child="asChild"
    aria-hidden="true"
    :data-state="itemContext.isExpanded.value ? 'open' : 'closed'"
    data-vize-tree-view-item-indicator
  >
    <slot />
  </Primitive>
</template>
