import type { PrimitiveProps } from '../Primitive'
import { createContext } from '../shared'

export interface CollapsibleRootProps extends PrimitiveProps {
  modelValue?: boolean
  defaultValue?: boolean
  disabled?: boolean
}

export interface CollapsibleRootContext {
  contentId: string
  open: import('vue').ComputedRef<boolean>
  disabled: boolean
  toggle: () => void
}

const _collapsibleRootContext = createContext<CollapsibleRootContext>('CollapsibleRoot')
export const injectCollapsibleRootContext = _collapsibleRootContext[0]
export const provideCollapsibleRootContext = _collapsibleRootContext[1]

export interface CollapsibleTriggerProps extends PrimitiveProps {}

export interface CollapsibleContentProps extends PrimitiveProps {
  forceMount?: boolean
}
