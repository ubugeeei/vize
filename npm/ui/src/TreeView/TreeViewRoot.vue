<script lang="ts">
export type { TreeViewRootProps, TreeViewRootContext } from './types'
export { injectTreeViewRootContext, provideTreeViewRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Primitive } from '../Primitive'
import { useDirection, kbd } from '../shared'
import type { TreeViewRootProps } from './types'
import { provideTreeViewRootContext } from './types'

const {
  as = 'div',
  asChild = false,
  modelValue,
  defaultValue,
  expanded,
  defaultExpanded,
  multiple = false,
  disabled = false,
  dir: dirProp,
  selectionMode = 'single',
} = defineProps<TreeViewRootProps>()

const emit = defineEmits<{
  'update:modelValue': [value: string[]]
  'update:expanded': [value: string[]]
}>()

const direction = useDirection(computed(() => dirProp))

const initSelected: string[] = defaultValue ?? []
const internalSelected = ref(initSelected)
const initExpanded: string[] = defaultExpanded ?? []
const internalExpanded = ref(initExpanded)

const selectedIds = computed(() => {
  const v: string[] = modelValue !== undefined ? modelValue : internalSelected.value
  return v
})

const expandedIds = computed(() => {
  const v: string[] = expanded !== undefined ? expanded : internalExpanded.value
  return v
})

function selectNode(id: string) {
  if (disabled) return
  if (selectionMode === 'none') return

  let next: string[]
  if (selectionMode === 'multiple' || multiple) {
    const current = selectedIds.value
    next = current.includes(id)
      ? current.filter(v => v !== id)
      : [...current, id]
  } else {
    next = selectedIds.value.includes(id) ? [] : [id]
  }
  internalSelected.value = next
  emit('update:modelValue', next)
}

function toggleExpanded(id: string) {
  if (disabled) return
  const current = expandedIds.value
  const next = current.includes(id)
    ? current.filter(v => v !== id)
    : [...current, id]
  internalExpanded.value = next
  emit('update:expanded', next)
}

function isSelected(id: string): boolean {
  return selectedIds.value.includes(id)
}

function isExpanded(id: string): boolean {
  return expandedIds.value.includes(id)
}

const initFocusedId: string | undefined = undefined
const focusedId = ref(initFocusedId)
const initFirstItemId: string | undefined = undefined
const firstItemId = ref(initFirstItemId)

function registerItem(id: string) {
  if (firstItemId.value === undefined) {
    firstItemId.value = id
  }
}

function setFocused(id: string) {
  focusedId.value = id
}

const initRootRef: HTMLElement | undefined = undefined
const rootRef = ref(initRootRef)

function getVisibleItems(): HTMLElement[] {
  if (!rootRef.value) return []
  const all = Array.from(
    rootRef.value.querySelectorAll('[role="treeitem"]'),
  )
  return all.filter((el) => {
    // Check if any ancestor group is hidden (collapsed)
    let parent = el.parentElement
    while (parent && parent !== rootRef.value) {
      if (parent.getAttribute('role') === 'group' && parent.style.display === 'none') {
        return false
      }
      parent = parent.parentElement
    }
    return true
  })
}

function expandAllSiblings(currentEl: HTMLElement) {
  const parent = currentEl.closest('[role="group"]') ?? rootRef.value
  if (!parent) return
  const siblings = Array.from(
    parent.querySelectorAll(':scope > [data-value]'),
  )
  for (const sibling of siblings) {
    const val = sibling.getAttribute('data-value')
    if (val && !isExpanded(val)) {
      toggleExpanded(val)
    }
  }
}

function handleKeydown(event: KeyboardEvent) {
  const items = getVisibleItems()
  if (items.length === 0) return

  const currentIndex = items.findIndex(
    el => el.getAttribute('data-value') === focusedId.value,
  )

  const currentEl = currentIndex >= 0 ? items[currentIndex] : null
  const currentValue = currentEl?.getAttribute('data-value')

  const isRtl = direction.value === 'rtl'
  const expandKey = isRtl ? kbd.ARROW_LEFT : kbd.ARROW_RIGHT
  const collapseKey = isRtl ? kbd.ARROW_RIGHT : kbd.ARROW_LEFT

  switch (event.key) {
    case kbd.ARROW_DOWN: {
      event.preventDefault()
      const nextIndex = currentIndex + 1
      if (nextIndex < items.length) {
        const next = items[nextIndex]
        const val = next.getAttribute('data-value')
        if (val) {
          setFocused(val)
          next.focus()
        }
      }
      break
    }
    case kbd.ARROW_UP: {
      event.preventDefault()
      const prevIndex = currentIndex - 1
      if (prevIndex >= 0) {
        const prev = items[prevIndex]
        const val = prev.getAttribute('data-value')
        if (val) {
          setFocused(val)
          prev.focus()
        }
      }
      break
    }
    case expandKey: {
      event.preventDefault()
      if (!currentValue) break
      const hasChildren = currentEl?.getAttribute('aria-expanded') !== null
      if (hasChildren && !isExpanded(currentValue)) {
        toggleExpanded(currentValue)
      } else if (hasChildren && isExpanded(currentValue)) {
        // Move to first child
        const refreshedItems = getVisibleItems()
        const refreshedIndex = refreshedItems.findIndex(
          el => el.getAttribute('data-value') === currentValue,
        )
        const nextChild = refreshedItems[refreshedIndex + 1]
        if (nextChild) {
          const val = nextChild.getAttribute('data-value')
          if (val) {
            setFocused(val)
            nextChild.focus()
          }
        }
      }
      break
    }
    case collapseKey: {
      event.preventDefault()
      if (!currentValue) break
      if (isExpanded(currentValue)) {
        toggleExpanded(currentValue)
      } else {
        // Move to parent
        const parentGroup = currentEl?.parentElement?.closest('[role="treeitem"]')
        if (parentGroup) {
          const parentValue = parentGroup.getAttribute('data-value')
          if (parentValue) {
            setFocused(parentValue)
            if (parentGroup instanceof HTMLElement) parentGroup.focus()
          }
        }
      }
      break
    }
    case kbd.HOME: {
      event.preventDefault()
      const first = items[0]
      if (first) {
        const val = first.getAttribute('data-value')
        if (val) {
          setFocused(val)
          first.focus()
        }
      }
      break
    }
    case kbd.END: {
      event.preventDefault()
      const last = items[items.length - 1]
      if (last) {
        const val = last.getAttribute('data-value')
        if (val) {
          setFocused(val)
          last.focus()
        }
      }
      break
    }
    case kbd.ENTER:
    case kbd.SPACE: {
      event.preventDefault()
      if (currentValue) {
        selectNode(currentValue)
      }
      break
    }
    case '*': {
      event.preventDefault()
      if (currentEl) {
        expandAllSiblings(currentEl)
      }
      break
    }
  }
}

provideTreeViewRootContext({
  selectedIds,
  expandedIds,
  disabled,
  multiple,
  dir: direction,
  selectionMode,
  selectNode,
  toggleExpanded,
  isSelected,
  isExpanded,
  focusedId,
  firstItemId,
  registerItem,
  setFocused,
  rootRef,
})
</script>

<template>
  <Primitive
    :ref="(el) => { rootRef = el?.$el ?? el }"
    :as="as"
    :as-child="asChild"
    role="tree"
    :aria-multiselectable="multiple || selectionMode === 'multiple' || undefined"
    data-orientation="vertical"
    data-vize-tree-view
    @keydown="handleKeydown"
  >
    <slot />
  </Primitive>
</template>
