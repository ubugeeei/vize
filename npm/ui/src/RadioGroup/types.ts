import type { PrimitiveProps } from '../Primitive'
import type { Direction, Orientation } from '../shared'
import { createContext } from '../shared'

export interface RadioGroupRootProps extends PrimitiveProps {
  modelValue?: string
  defaultValue?: string
  disabled?: boolean
  required?: boolean
  name?: string
  orientation?: Orientation
  loop?: boolean
  dir?: Direction
}

export interface RadioGroupRootContext {
  modelValue: import('vue').ComputedRef<string | undefined>
  disabled: boolean
  required: boolean
  name: string | undefined
  orientation: Orientation
  loop: boolean
  dir: Direction
  updateValue: (value: string) => void
}

const _radioGroupRootContext = createContext<RadioGroupRootContext>('RadioGroupRoot')
export const injectRadioGroupRootContext = _radioGroupRootContext[0]
export const provideRadioGroupRootContext = _radioGroupRootContext[1]

export interface RadioGroupItemProps extends PrimitiveProps {
  value: string
  disabled?: boolean
}

export interface RadioGroupItemContext {
  checked: import('vue').ComputedRef<boolean>
  disabled: boolean
}

const _radioGroupItemContext = createContext<RadioGroupItemContext>('RadioGroupItem')
export const injectRadioGroupItemContext = _radioGroupItemContext[0]
export const provideRadioGroupItemContext = _radioGroupItemContext[1]

export interface RadioGroupIndicatorProps extends PrimitiveProps {}
