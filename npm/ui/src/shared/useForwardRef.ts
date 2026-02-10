import type { Ref } from 'vue'
import { type ComponentPublicInstance, ref } from 'vue'

export function useForwardRef() {
  const forwardRef = ref<HTMLElement>() as Ref<HTMLElement | undefined>

  function setRef(el: Element | ComponentPublicInstance | null) {
    if (el instanceof Element) {
      forwardRef.value = el as HTMLElement
    } else if (el && '$el' in el) {
      forwardRef.value = el.$el as HTMLElement
    } else {
      forwardRef.value = undefined
    }
  }

  return { forwardRef, setRef }
}
