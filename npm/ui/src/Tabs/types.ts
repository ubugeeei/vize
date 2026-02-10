import type { ComputedRef, Ref } from 'vue'
import type { PrimitiveProps } from '../Primitive'
import type { Direction, Orientation } from '../shared'
import { createContext } from '../shared'

export type ActivationMode = 'automatic' | 'manual'

export interface TabsRootProps extends PrimitiveProps {
  modelValue?: string
  defaultValue?: string
  orientation?: Orientation
  dir?: Direction
  activationMode?: ActivationMode
}

export interface TabsRootContext {
  modelValue: ComputedRef<string>
  orientation: Orientation
  dir: ComputedRef<Direction>
  activationMode: ActivationMode
  changeValue: (value: string) => void
  parentRef: Ref<HTMLElement | undefined>
  baseId: string
}

const _tabsRootContext = createContext<TabsRootContext>('TabsRoot')
export const injectTabsRootContext = _tabsRootContext[0]
export const provideTabsRootContext = _tabsRootContext[1]

export interface TabsListProps extends PrimitiveProps {
  loop?: boolean
}

export interface TabsTriggerProps extends PrimitiveProps {
  value: string
  disabled?: boolean
}

export interface TabsContentProps extends PrimitiveProps {
  value: string
  forceMount?: boolean
}

export interface TabsIndicatorProps extends PrimitiveProps {}
