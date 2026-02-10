import type { VNode } from 'vue'
import { Fragment } from 'vue'

export function getActiveElement(doc?: Document): HTMLElement | null {
  const d = doc ?? (typeof document !== 'undefined' ? document : undefined)
  if (!d) return null

  let activeElement = d.activeElement as HTMLElement | null
  while (activeElement?.shadowRoot?.activeElement) {
    activeElement = activeElement.shadowRoot.activeElement as HTMLElement
  }
  return activeElement
}

export function renderSlotFragments(children: VNode[]): VNode[] {
  return children.flatMap((child) => {
    if (child.type === Fragment) {
      return renderSlotFragments(child.children as VNode[])
    }
    return [child]
  })
}

export function focusFirst(candidates: HTMLElement[], preventScroll = false) {
  const previouslyFocused = getActiveElement()

  for (const candidate of candidates) {
    if (candidate === previouslyFocused) return
    candidate.focus({ preventScroll })
    if (getActiveElement() !== previouslyFocused) return
  }
}

export function getTabbableElements(container: HTMLElement): HTMLElement[] {
  const tabbable: HTMLElement[] = []
  const walker = document.createTreeWalker(
    container,
    NodeFilter.SHOW_ELEMENT,
    {
      acceptNode(node: HTMLElement) {
        if (node.tagName === 'INPUT' && (node as HTMLInputElement).type === 'hidden') {
          return NodeFilter.FILTER_SKIP
        }
        if (node.tabIndex >= 0 && !node.hasAttribute('disabled')) {
          return NodeFilter.FILTER_ACCEPT
        }
        return NodeFilter.FILTER_SKIP
      },
    },
  )

  while (walker.nextNode()) {
    tabbable.push(walker.currentNode as HTMLElement)
  }
  return tabbable
}
