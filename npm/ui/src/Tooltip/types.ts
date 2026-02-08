import type { PrimitiveProps } from '../Primitive'
import { createContext } from '../shared'

export interface TooltipProviderProps {
  delayDuration?: number
  skipDelayDuration?: number
  disableHoverableContent?: boolean
  disableClosingTrigger?: boolean
}

export interface TooltipProviderContext {
  delayDuration: number
  skipDelayDuration: number
  disableHoverableContent: boolean
  disableClosingTrigger: boolean
  isOpenDelayed: import('vue').Ref<boolean>
  onOpen: () => void
  onClose: () => void
}

const _tooltipProviderContext = createContext<TooltipProviderContext>('TooltipProvider')
export const injectTooltipProviderContext = _tooltipProviderContext[0]
export const provideTooltipProviderContext = _tooltipProviderContext[1]

export interface TooltipRootProps {
  modelValue?: boolean
  defaultValue?: boolean
  delayDuration?: number
}

export interface TooltipRootContext {
  open: import('vue').ComputedRef<boolean>
  onOpenChange: (value: boolean) => void
  triggerRef: import('vue').Ref<HTMLElement | undefined>
  contentId: string
  isDisabled: boolean
}

const _tooltipRootContext = createContext<TooltipRootContext>('TooltipRoot')
export const injectTooltipRootContext = _tooltipRootContext[0]
export const provideTooltipRootContext = _tooltipRootContext[1]

export interface TooltipTriggerProps extends PrimitiveProps {}

export interface TooltipPortalProps {
  to?: string
  disabled?: boolean
}

export type TooltipContentSide = 'top' | 'right' | 'bottom' | 'left'
export type TooltipContentAlign = 'start' | 'center' | 'end'

export interface TooltipContentProps extends PrimitiveProps {
  forceMount?: boolean
  side?: TooltipContentSide
  sideOffset?: number
  align?: TooltipContentAlign
  alignOffset?: number
}

export type TooltipContentEmits = {
  'escapeKeyDown': [event: KeyboardEvent]
  'pointerDownOutside': [event: PointerEvent]
}

export interface TooltipArrowProps extends PrimitiveProps {
  width?: number
  height?: number
}
