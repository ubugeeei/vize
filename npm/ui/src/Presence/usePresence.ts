import { type Ref, computed, ref, watch } from 'vue'
import { useStateMachine } from '../shared'

export function usePresence(present: Ref<boolean>) {
  const { state, dispatch } = useStateMachine('mounted', {
    mounted: {
      UNMOUNT: 'unmounted',
      ANIMATION_OUT: 'unmountSuspended',
    },
    unmountSuspended: {
      MOUNT: 'mounted',
      ANIMATION_END: 'unmounted',
    },
    unmounted: {
      MOUNT: 'mounted',
    },
  })

  const stylesRef = ref<CSSStyleDeclaration>()
  const prevPresentRef = ref(present.value)
  const prevAnimationNameRef = ref<string>('none')
  const nodeRef = ref<HTMLElement>()

  function setNodeRef(node: HTMLElement | undefined) {
    if (node) {
      nodeRef.value = node
      stylesRef.value = getComputedStyle(node)
    }
  }

  watch(present, (currentPresent) => {
    const prevPresent = prevPresentRef.value

    if (prevPresent !== currentPresent) {
      const hasPendingAnimation = prevAnimationNameRef.value !== 'none'
      const currentAnimationName = stylesRef.value?.animationName ?? 'none'

      if (currentPresent) {
        dispatch('MOUNT')
      } else if (currentAnimationName === 'none' || !hasPendingAnimation) {
        dispatch('UNMOUNT')
      } else {
        dispatch('ANIMATION_OUT')
      }

      prevPresentRef.value = currentPresent
    }
  })

  function handleAnimationStart(event: AnimationEvent) {
    if (event.target === nodeRef.value) {
      prevAnimationNameRef.value = getAnimationName(stylesRef.value)
    }
  }

  function handleAnimationEnd(event: AnimationEvent) {
    const currentAnimationName = getAnimationName(stylesRef.value)
    const isCurrentAnimation = currentAnimationName.includes(event.animationName)

    if (event.target === nodeRef.value && isCurrentAnimation) {
      dispatch('ANIMATION_END')
    }
  }

  const isPresent = computed(() => {
    return state.value === 'mounted' || state.value === 'unmountSuspended'
  })

  return {
    isPresent,
    ref: setNodeRef,
    onAnimationStart: handleAnimationStart,
    onAnimationEnd: handleAnimationEnd,
  }
}

function getAnimationName(styles?: CSSStyleDeclaration): string {
  return styles?.animationName ?? 'none'
}
