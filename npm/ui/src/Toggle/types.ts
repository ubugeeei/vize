import type { PrimitiveProps } from '../Primitive'

export interface ToggleRootProps extends PrimitiveProps {
  modelValue?: boolean
  defaultValue?: boolean
  disabled?: boolean
}
