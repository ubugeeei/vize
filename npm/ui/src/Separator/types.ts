import type { PrimitiveProps } from '../Primitive'
import type { Orientation } from '../shared'

export interface SeparatorProps extends PrimitiveProps {
  orientation?: Orientation
  decorative?: boolean
}
