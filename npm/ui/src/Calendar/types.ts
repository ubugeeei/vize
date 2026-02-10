import type { ComputedRef, Ref } from 'vue'
import type { PrimitiveProps } from '../Primitive'
import type { Direction } from '../shared'
import { createContext } from '../shared'

export type DateValue = Date

export interface CalendarRootProps extends PrimitiveProps {
  modelValue?: DateValue | DateValue[]
  defaultValue?: DateValue | DateValue[]
  minDate?: DateValue
  maxDate?: DateValue
  disabled?: boolean
  readonly?: boolean
  multiple?: boolean
  dir?: Direction
  locale?: string
  weekStartsOn?: 0 | 1 | 2 | 3 | 4 | 5 | 6
  numberOfMonths?: number
  fixedWeeks?: boolean
  isDateDisabled?: (date: DateValue) => boolean
  isDateUnavailable?: (date: DateValue) => boolean
}

export interface CalendarDay {
  date: DateValue
  isToday: boolean
  isSelected: boolean
  isDisabled: boolean
  isUnavailable: boolean
  isOutsideMonth: boolean
}

export interface CalendarMonth {
  year: number
  month: number
  weeks: CalendarDay[][]
}

export interface CalendarRootContext {
  modelValue: ComputedRef<DateValue[]>
  focusedDate: Ref<DateValue>
  months: ComputedRef<CalendarMonth[]>
  weekDays: ComputedRef<string[]>
  locale: string
  disabled: boolean
  readonly: boolean
  multiple: boolean
  dir: ComputedRef<Direction>
  isDateDisabled: (date: DateValue) => boolean
  isDateUnavailable: (date: DateValue) => boolean
  isDateSelected: (date: DateValue) => boolean
  selectDate: (date: DateValue) => void
  focusDate: (date: DateValue) => void
  nextMonth: () => void
  prevMonth: () => void
  nextYear: () => void
  prevYear: () => void
}

const _calendarRootContext = createContext<CalendarRootContext>('CalendarRoot')
export const injectCalendarRootContext = _calendarRootContext[0]
export const provideCalendarRootContext = _calendarRootContext[1]

export interface CalendarHeaderProps extends PrimitiveProps {}
export interface CalendarPrevProps extends PrimitiveProps {}
export interface CalendarNextProps extends PrimitiveProps {}
export interface CalendarHeadingProps extends PrimitiveProps {}
export interface CalendarGridProps extends PrimitiveProps {}
export interface CalendarGridHeadProps extends PrimitiveProps {}
export interface CalendarGridHeadCellProps extends PrimitiveProps {}
export interface CalendarGridBodyProps extends PrimitiveProps {}
export interface CalendarCellProps extends PrimitiveProps {
  date: DateValue
}
export interface CalendarCellTriggerProps extends PrimitiveProps {
  date: DateValue
}
