<script lang="ts">
export type { TabsListProps } from './types'
</script>

<script setup lang="ts">
import { ref } from 'vue'
import { Primitive } from '../Primitive'
import { useArrowNavigation } from '../shared'
import { injectTabsRootContext } from './types'
import type { TabsListProps } from './types'

const { as = 'div', asChild = false, loop = true } = defineProps<TabsListProps>()

const rootContext = injectTabsRootContext('TabsList')
const initListRef: HTMLElement | undefined = undefined
const listRef = ref(initListRef)

function handleKeydown(event: KeyboardEvent) {
  const target = useArrowNavigation(
    event,
    event.currentTarget === event.target ? null : (event.target instanceof HTMLElement ? event.target : null),
    listRef.value,
    {
      orientation: rootContext.orientation,
      loop,
      dir: rootContext.dir.value,
    },
  )

  if (target) {
    event.preventDefault()
    target.focus()

    if (rootContext.activationMode === 'automatic') {
      const triggerValue = target.getAttribute('data-value')
      if (triggerValue) {
        rootContext.changeValue(triggerValue)
      }
    }
  }
}
</script>

<template>
  <Primitive
    :ref="(el) => { listRef = el?.$el ?? el }"
    :as="as"
    :as-child="asChild"
    role="tablist"
    :aria-orientation="rootContext.orientation"
    :data-orientation="rootContext.orientation"
    data-vize-tabs-list
    @keydown="handleKeydown"
  >
    <slot />
  </Primitive>
</template>
