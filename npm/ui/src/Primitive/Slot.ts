import {
  cloneVNode,
  defineComponent,
  mergeProps,
} from 'vue'
import { renderSlotFragments } from '../shared'

function mergeEventHandlers(
  existing: ((...args: unknown[]) => void) | undefined,
  incoming: ((...args: unknown[]) => void) | undefined,
) {
  if (!existing) return incoming
  if (!incoming) return existing
  return (...args: unknown[]) => {
    incoming(...args)
    existing(...args)
  }
}

export const Slot = defineComponent({
  name: 'Slot',
  inheritAttrs: false,
  setup(_, { slots, attrs }) {
    return () => {
      const defaultSlot = slots.default?.()
      if (!defaultSlot) return null

      const children = renderSlotFragments(defaultSlot)
      const firstNonTextChild = children.find(
        (child) => typeof child.type !== 'symbol',
      )

      if (!firstNonTextChild) return children

      // Merge attrs onto the first child
      const mergedProps: Record<string, unknown> = {}
      const childProps = (firstNonTextChild.props ?? {}) as Record<string, unknown>

      for (const [key, value] of Object.entries(attrs)) {
        if (key.startsWith('on') && typeof value === 'function') {
          mergedProps[key] = mergeEventHandlers(
            childProps[key] as ((...args: unknown[]) => void) | undefined,
            value as (...args: unknown[]) => void,
          )
        }
      }

      const cloned = cloneVNode(
        firstNonTextChild,
        mergeProps(attrs, mergedProps),
      )

      // Replace first child with cloned version
      for (let i = 0; i < children.length; i++) {
        if (children[i] === firstNonTextChild) {
          children[i] = cloned
          break
        }
      }

      return children.length === 1 ? children[0] : children
    }
  },
})
