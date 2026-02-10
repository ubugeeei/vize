import type { PrimitiveProps } from '../Primitive'
import { createContext } from '../shared'

export interface PopoverRootProps {
  modelValue?: boolean
  defaultValue?: boolean
  modal?: boolean
}

export interface PopoverRootContext {
  open: import('vue').ComputedRef<boolean>
  onOpenChange: (value: boolean) => void
  triggerRef: import('vue').Ref<HTMLElement | undefined>
  contentId: string
  modal: boolean
}

const _popoverRootContext = createContext<PopoverRootContext>('PopoverRoot')
export const injectPopoverRootContext = _popoverRootContext[0]
export const providePopoverRootContext = _popoverRootContext[1]

export interface PopoverTriggerProps extends PrimitiveProps {}

export interface PopoverPortalProps {
  to?: string
  disabled?: boolean
}

export type PopoverContentSide = 'top' | 'right' | 'bottom' | 'left'
export type PopoverContentAlign = 'start' | 'center' | 'end'

export interface PopoverContentProps extends PrimitiveProps {
  forceMount?: boolean
  side?: PopoverContentSide
  sideOffset?: number
  align?: PopoverContentAlign
  alignOffset?: number
  trapFocus?: boolean
}

export type PopoverContentEmits = {
  'escapeKeyDown': [event: KeyboardEvent]
  'pointerDownOutside': [event: PointerEvent]
  'focusOutside': [event: FocusEvent]
  'interactOutside': [event: Event]
  'openAutoFocus': [event: Event]
  'closeAutoFocus': [event: Event]
}

export interface PopoverCloseProps extends PrimitiveProps {}

export interface PopoverArrowProps extends PrimitiveProps {
  width?: number
  height?: number
}
