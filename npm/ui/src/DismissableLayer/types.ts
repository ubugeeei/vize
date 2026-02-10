import type { PrimitiveProps } from '../Primitive'

export interface DismissableLayerProps extends PrimitiveProps {
  disableOutsidePointerEvents?: boolean
}

export type DismissableLayerEmits = {
  'escapeKeyDown': [event: KeyboardEvent]
  'pointerDownOutside': [event: PointerEvent]
  'focusOutside': [event: FocusEvent]
  'interactOutside': [event: Event]
  'dismiss': []
}
