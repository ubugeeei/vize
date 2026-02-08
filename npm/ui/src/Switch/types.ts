import type { PrimitiveProps } from '../Primitive'
import { createContext } from '../shared'

export interface SwitchRootProps extends PrimitiveProps {
  modelValue?: boolean
  defaultValue?: boolean
  disabled?: boolean
  required?: boolean
  name?: string
  value?: string
}

export interface SwitchRootContext {
  checked: import('vue').ComputedRef<boolean>
  disabled: boolean
}

const _switchRootContext = createContext<SwitchRootContext>('SwitchRoot')
export const injectSwitchRootContext = _switchRootContext[0]
export const provideSwitchRootContext = _switchRootContext[1]

export interface SwitchThumbProps extends PrimitiveProps {}
