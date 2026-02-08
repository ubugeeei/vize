import type { DateValue } from './types'

export function isSameDay(a: DateValue, b: DateValue): boolean {
  return (
    a.getFullYear() === b.getFullYear()
    && a.getMonth() === b.getMonth()
    && a.getDate() === b.getDate()
  )
}

export function isSameMonth(a: DateValue, b: DateValue): boolean {
  return (
    a.getFullYear() === b.getFullYear()
    && a.getMonth() === b.getMonth()
  )
}

export function isToday(date: DateValue): boolean {
  return isSameDay(date, new Date())
}

export function isBeforeDay(a: DateValue, b: DateValue): boolean {
  const aDate = new Date(a.getFullYear(), a.getMonth(), a.getDate())
  const bDate = new Date(b.getFullYear(), b.getMonth(), b.getDate())
  return aDate.getTime() < bDate.getTime()
}

export function isAfterDay(a: DateValue, b: DateValue): boolean {
  const aDate = new Date(a.getFullYear(), a.getMonth(), a.getDate())
  const bDate = new Date(b.getFullYear(), b.getMonth(), b.getDate())
  return aDate.getTime() > bDate.getTime()
}

export function addDays(date: DateValue, days: number): DateValue {
  const result = new Date(date)
  result.setDate(result.getDate() + days)
  return result
}

export function addMonths(date: DateValue, months: number): DateValue {
  const result = new Date(date)
  const targetMonth = result.getMonth() + months
  result.setMonth(targetMonth)
  // Handle month overflow (e.g., Jan 31 + 1 month should be Feb 28)
  if (result.getMonth() !== ((targetMonth % 12) + 12) % 12) {
    result.setDate(0) // go to last day of previous month
  }
  return result
}

export function startOfMonth(date: DateValue): DateValue {
  return new Date(date.getFullYear(), date.getMonth(), 1)
}

export function endOfMonth(date: DateValue): DateValue {
  return new Date(date.getFullYear(), date.getMonth() + 1, 0)
}

export function startOfWeek(date: DateValue, weekStartsOn: number = 1): DateValue {
  const result = new Date(date)
  const day = result.getDay()
  const diff = (day - weekStartsOn + 7) % 7
  result.setDate(result.getDate() - diff)
  return result
}

export function getDaysInMonth(date: DateValue): number {
  return new Date(date.getFullYear(), date.getMonth() + 1, 0).getDate()
}

export function getMonthGrid(
  year: number,
  month: number,
  weekStartsOn: number = 1,
  fixedWeeks: boolean = false,
): Date[][] {
  const firstDay = new Date(year, month, 1)
  const weekStart = startOfWeek(firstDay, weekStartsOn)

  const weeks: Date[][] = []
  let current = new Date(weekStart)

  // Generate at least enough weeks to cover the month
  const targetWeeks = fixedWeeks ? 6 : undefined

  while (true) {
    const week: Date[] = []
    for (let i = 0; i < 7; i++) {
      week.push(new Date(current))
      current = addDays(current, 1)
    }
    weeks.push(week)

    if (targetWeeks) {
      if (weeks.length >= targetWeeks) break
    } else {
      // Check if we've covered the entire month
      // We need at least one week, and the last added day of the previous week
      // should be past the end of month
      const lastDayInWeek = week[6]
      if (
        lastDayInWeek.getMonth() !== month
        || lastDayInWeek.getDate() === getDaysInMonth(firstDay)
      ) {
        // If fixedWeeks is not set but we haven't filled 6 weeks, stop
        break
      }
    }
  }

  return weeks
}

export function getWeekDayNames(locale: string = 'en-US', weekStartsOn: number = 1): string[] {
  const names: string[] = []
  // Use a known date (2024-01-01 is a Monday) as a reference
  // We'll pick a Sunday: 2024-01-07
  const baseSunday = new Date(2024, 0, 7) // Sunday

  for (let i = 0; i < 7; i++) {
    const dayIndex = (weekStartsOn + i) % 7
    const date = new Date(baseSunday)
    date.setDate(date.getDate() + dayIndex)
    names.push(
      date.toLocaleDateString(locale, { weekday: 'short' }),
    )
  }

  return names
}

export function formatMonth(date: DateValue, locale: string = 'en-US'): string {
  return date.toLocaleDateString(locale, { month: 'long', year: 'numeric' })
}

export function formatFullDate(date: DateValue, locale: string = 'en-US'): string {
  return date.toLocaleDateString(locale, {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })
}

export function getWeekDayFullNames(locale: string = 'en-US', weekStartsOn: number = 1): string[] {
  const names: string[] = []
  const baseSunday = new Date(2024, 0, 7)

  for (let i = 0; i < 7; i++) {
    const dayIndex = (weekStartsOn + i) % 7
    const date = new Date(baseSunday)
    date.setDate(date.getDate() + dayIndex)
    names.push(
      date.toLocaleDateString(locale, { weekday: 'long' }),
    )
  }

  return names
}
