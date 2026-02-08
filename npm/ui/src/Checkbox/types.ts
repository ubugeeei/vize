import type { PrimitiveProps } from '../Primitive'
import { createContext } from '../shared'

export type CheckedState = boolean | 'indeterminate'

export interface CheckboxRootProps extends PrimitiveProps {
  modelValue?: CheckedState
  defaultValue?: CheckedState
  disabled?: boolean
  required?: boolean
  name?: string
  value?: string
}

export interface CheckboxRootContext {
  state: import('vue').ComputedRef<CheckedState>
  disabled: boolean
}

const _checkboxRootContext = createContext<CheckboxRootContext>('CheckboxRoot')
export const injectCheckboxRootContext = _checkboxRootContext[0]
export const provideCheckboxRootContext = _checkboxRootContext[1]

export interface CheckboxIndicatorProps extends PrimitiveProps {}
