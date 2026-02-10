import { defineComponent, toRef } from 'vue'
import { usePresence } from './usePresence'

export const Presence = defineComponent({
  name: 'Presence',
  props: {
    present: { type: Boolean, required: true },
  },
  setup(props, { slots }) {
    const presentRef = toRef(props, 'present')
    const { isPresent, ref: presenceRef, onAnimationStart, onAnimationEnd } = usePresence(presentRef)

    return () => {
      if (!isPresent.value) return null

      return slots.default?.({
        isPresent: isPresent.value,
        ref: presenceRef,
        onAnimationStart,
        onAnimationEnd,
      })
    }
  },
})
