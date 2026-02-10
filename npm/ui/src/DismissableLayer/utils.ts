export type DismissableLayerElement = HTMLElement

export interface LayerEntry {
  element: DismissableLayerElement
  disableOutsidePointerEvents: boolean
}

export const layerStack = new Set<LayerEntry>()

export function getTopLayer(): LayerEntry | undefined {
  return [...layerStack].pop()
}

export function isTopLayer(element: DismissableLayerElement): boolean {
  const topLayer = getTopLayer()
  return topLayer?.element === element
}

export function hasDisabledOutsidePointerEvents(): boolean {
  return [...layerStack].some((layer) => layer.disableOutsidePointerEvents)
}

export function isPointerDownOutside(
  event: PointerEvent,
  element: DismissableLayerElement,
): boolean {
  const target = event.target as Node
  if (!target) return false
  return !element.contains(target)
}

export function isFocusOutside(
  event: FocusEvent,
  element: DismissableLayerElement,
): boolean {
  const relatedTarget = event.relatedTarget as Node | null
  if (!relatedTarget) return true
  return !element.contains(relatedTarget)
}
