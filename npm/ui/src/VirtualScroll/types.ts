import type { PrimitiveProps } from '../Primitive'
import type { Ref, ComputedRef } from 'vue'
import { createContext } from '../shared'

export interface VirtualScrollProps extends PrimitiveProps {
  items: unknown[]
  itemHeight: number | ((index: number) => number)
  overscan?: number
  horizontal?: boolean
}

export interface VirtualItem {
  index: number
  start: number
  size: number
  item: unknown
}

export interface VirtualScrollContext {
  virtualItems: ComputedRef<VirtualItem[]>
  totalSize: ComputedRef<number>
  scrollOffset: Ref<number>
  containerRef: Ref<HTMLElement | undefined>
}

const _virtualScrollContext = createContext<VirtualScrollContext>('VirtualScroll')
export const injectVirtualScrollContext = _virtualScrollContext[0]
export const provideVirtualScrollContext = _virtualScrollContext[1]

export interface VirtualScrollContentProps extends PrimitiveProps {}
