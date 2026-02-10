import type { ComputedRef, Ref } from 'vue'
import type { PrimitiveProps } from '../Primitive'
import type { Direction } from '../shared'
import type { DateValue } from '../Calendar/types'
import { createContext } from '../shared'

export interface DatePickerRootProps {
  modelValue?: DateValue
  defaultValue?: DateValue
  minDate?: DateValue
  maxDate?: DateValue
  disabled?: boolean
  readonly?: boolean
  dir?: Direction
  locale?: string
  weekStartsOn?: 0 | 1 | 2 | 3 | 4 | 5 | 6
  placeholder?: string
  formatDate?: (date: DateValue) => string
  isDateDisabled?: (date: DateValue) => boolean
}

export interface DatePickerRootContext {
  modelValue: ComputedRef<DateValue | undefined>
  open: Ref<boolean>
  disabled: boolean
  readonly: boolean
  locale: string
  formatDate: (date: DateValue) => string
  onDateSelect: (date: DateValue) => void
  onOpenChange: (open: boolean) => void
  triggerRef: Ref<HTMLElement | undefined>
  contentId: string
}

const _datePickerRootContext = createContext<DatePickerRootContext>('DatePickerRoot')
export const injectDatePickerRootContext = _datePickerRootContext[0]
export const provideDatePickerRootContext = _datePickerRootContext[1]

export interface DatePickerTriggerProps extends PrimitiveProps {}
export interface DatePickerContentProps extends PrimitiveProps {
  forceMount?: boolean
  to?: string | HTMLElement
  disableTeleport?: boolean
}
export interface DatePickerInputProps extends PrimitiveProps {
  placeholder?: string
}
