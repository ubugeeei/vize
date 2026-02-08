import { type Component, defineComponent, h } from 'vue'
import { Slot } from './Slot'

export interface PrimitiveProps {
  as?: string | Component
  asChild?: boolean
}

const SELF_CLOSING_TAGS = new Set(['area', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'source', 'track', 'wbr'])

export const Primitive = defineComponent({
  name: 'Primitive',
  inheritAttrs: false,
  props: {
    as: { type: [String, Object], default: 'div' },
    asChild: { type: Boolean, default: false },
  },
  setup(props, { attrs, slots }) {
    return () => {
      if (props.asChild) {
        return h(Slot, attrs, slots)
      }

      const { as: tag } = props
      if (typeof tag === 'string' && SELF_CLOSING_TAGS.has(tag)) {
        return h(tag, attrs)
      }

      return h(tag, attrs, { default: slots.default })
    }
  },
})
