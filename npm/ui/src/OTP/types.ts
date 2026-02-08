import type { ComputedRef, Ref } from 'vue'
import type { PrimitiveProps } from '../Primitive'
import { createContext } from '../shared'

export type OTPInputMode = 'numeric' | 'text'

export interface OTPRootProps extends PrimitiveProps {
  modelValue?: string
  defaultValue?: string
  length?: number
  disabled?: boolean
  inputMode?: OTPInputMode
  mask?: boolean | string
  autoSubmit?: boolean
}

export interface OTPRootContext {
  value: ComputedRef<string>
  length: number
  disabled: boolean
  inputMode: OTPInputMode
  mask: boolean | string
  focusedIndex: Ref<number>
  setChar: (index: number, char: string) => void
  deleteChar: (index: number) => void
  focusIndex: (index: number) => void
  isFilled: ComputedRef<boolean>
  isInputFocused: Ref<boolean>
}

const _otpRootContext = createContext<OTPRootContext>('OTPRoot')
export const injectOTPRootContext = _otpRootContext[0]
export const provideOTPRootContext = _otpRootContext[1]

export interface OTPSlotProps extends PrimitiveProps {
  index: number
}

export interface OTPSeparatorProps extends PrimitiveProps {}
