import type { PrimitiveProps } from '../Primitive'
import { createContext } from '../shared'

export interface DialogRootProps {
  modelValue?: boolean
  defaultValue?: boolean
}

export interface DialogRootContext {
  open: import('vue').ComputedRef<boolean>
  onOpenChange: (value: boolean) => void
  titleId: string
  descriptionId: string
  triggerRef: import('vue').Ref<HTMLElement | undefined>
  contentId: string
}

const _dialogRootContext = createContext<DialogRootContext>('DialogRoot')
export const injectDialogRootContext = _dialogRootContext[0]
export const provideDialogRootContext = _dialogRootContext[1]

export interface DialogTriggerProps extends PrimitiveProps {}

export interface DialogPortalProps {
  to?: string
  disabled?: boolean
}

export interface DialogOverlayProps extends PrimitiveProps {
  forceMount?: boolean
}

export interface DialogContentProps extends PrimitiveProps {
  forceMount?: boolean
  trapFocus?: boolean
}

export type DialogContentEmits = {
  'escapeKeyDown': [event: KeyboardEvent]
  'pointerDownOutside': [event: PointerEvent]
  'focusOutside': [event: FocusEvent]
  'interactOutside': [event: Event]
  'openAutoFocus': [event: Event]
  'closeAutoFocus': [event: Event]
}

export interface DialogTitleProps extends PrimitiveProps {}

export interface DialogDescriptionProps extends PrimitiveProps {}

export interface DialogCloseProps extends PrimitiveProps {}
