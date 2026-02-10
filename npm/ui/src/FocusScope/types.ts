import type { PrimitiveProps } from '../Primitive'

export interface FocusScopeProps extends PrimitiveProps {
  trapped?: boolean
  loop?: boolean
}

export type FocusScopeEmits = {
  'mountAutoFocus': [event: Event]
  'unmountAutoFocus': [event: Event]
}
