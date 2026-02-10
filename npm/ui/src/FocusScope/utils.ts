const focusableSelector = [
  'a[href]',
  'area[href]',
  'input:not([disabled]):not([type="hidden"])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  'button:not([disabled])',
  '[tabindex]',
  'iframe',
  'object',
  'embed',
  'audio[controls]',
  'video[controls]',
  '[contenteditable]:not([contenteditable="false"])',
  'details>summary:first-of-type',
  'details',
].join(',')

export { focusableSelector }

export function getFocusableElements(container: HTMLElement): HTMLElement[] {
  const elements = Array.from(
    container.querySelectorAll<HTMLElement>(focusableSelector),
  )
  return elements.filter((el) => {
    return !el.hasAttribute('disabled') && !el.closest('[hidden]') && el.tabIndex >= 0
  })
}

export function getFirstFocusable(container: HTMLElement): HTMLElement | null {
  const elements = getFocusableElements(container)
  return elements[0] ?? null
}

export function getLastFocusable(container: HTMLElement): HTMLElement | null {
  const elements = getFocusableElements(container)
  return elements[elements.length - 1] ?? null
}
