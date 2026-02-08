import type { Orientation } from './types'

export interface ArrowNavigationOptions {
  orientation?: Orientation
  loop?: boolean
  dir?: 'ltr' | 'rtl'
}

export function useArrowNavigation(
  event: KeyboardEvent,
  currentElement: HTMLElement | null,
  parentElement: HTMLElement | undefined,
  options: ArrowNavigationOptions = {},
): HTMLElement | null {
  const { orientation = 'horizontal', loop = true, dir = 'ltr' } = options

  if (!parentElement) return null

  const candidates = Array.from(
    parentElement.querySelectorAll<HTMLElement>(
      '[data-vize-collection-item]:not([data-disabled])',
    ),
  )

  if (candidates.length === 0) return null

  const currentIndex = currentElement
    ? candidates.indexOf(currentElement)
    : -1

  const isForward =
    orientation === 'horizontal'
      ? dir === 'ltr'
        ? event.key === 'ArrowRight'
        : event.key === 'ArrowLeft'
      : event.key === 'ArrowDown'

  const isBackward =
    orientation === 'horizontal'
      ? dir === 'ltr'
        ? event.key === 'ArrowLeft'
        : event.key === 'ArrowRight'
      : event.key === 'ArrowUp'

  const isHome = event.key === 'Home'
  const isEnd = event.key === 'End'

  if (isHome) return candidates[0] ?? null
  if (isEnd) return candidates[candidates.length - 1] ?? null

  if (isForward) {
    const next = currentIndex + 1
    return loop
      ? (candidates[next % candidates.length] ?? null)
      : (candidates[next] ?? null)
  }

  if (isBackward) {
    const prev = currentIndex - 1
    return loop
      ? (candidates[(prev + candidates.length) % candidates.length] ?? null)
      : (candidates[prev] ?? null)
  }

  return null
}
