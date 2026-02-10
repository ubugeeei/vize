import type { ComputedRef, Ref } from 'vue'
import type { PrimitiveProps } from '../Primitive'
import type { Direction } from '../shared'
import { createContext } from '../shared'

export interface TreeViewRootProps extends PrimitiveProps {
  modelValue?: string[]
  defaultValue?: string[]
  expanded?: string[]
  defaultExpanded?: string[]
  multiple?: boolean
  disabled?: boolean
  dir?: Direction
  selectionMode?: 'single' | 'multiple' | 'none'
}

export interface TreeViewRootContext {
  selectedIds: ComputedRef<string[]>
  expandedIds: ComputedRef<string[]>
  disabled: boolean
  multiple: boolean
  dir: ComputedRef<Direction>
  selectionMode: 'single' | 'multiple' | 'none'
  selectNode: (id: string) => void
  toggleExpanded: (id: string) => void
  isSelected: (id: string) => boolean
  isExpanded: (id: string) => boolean
  focusedId: Ref<string | undefined>
  firstItemId: Ref<string | undefined>
  registerItem: (id: string) => void
  setFocused: (id: string) => void
  rootRef: Ref<HTMLElement | undefined>
}

const _treeViewRootContext = createContext<TreeViewRootContext>('TreeViewRoot')
export const injectTreeViewRootContext = _treeViewRootContext[0]
export const provideTreeViewRootContext = _treeViewRootContext[1]

export interface TreeViewItemProps extends PrimitiveProps {
  value: string
  disabled?: boolean
}

export interface TreeViewItemContext {
  value: string
  level: number
  disabled: boolean
  isLeaf: Ref<boolean>
  isExpanded: ComputedRef<boolean>
  isSelected: ComputedRef<boolean>
}

const _treeViewItemContext = createContext<TreeViewItemContext>('TreeViewItem')
export const injectTreeViewItemContext = _treeViewItemContext[0]
export const provideTreeViewItemContext = _treeViewItemContext[1]

export interface TreeViewItemTriggerProps extends PrimitiveProps {}
export interface TreeViewItemIndicatorProps extends PrimitiveProps {}
export interface TreeViewGroupProps extends PrimitiveProps {}
