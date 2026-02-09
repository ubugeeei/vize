<script lang="ts">
export type { FocusScopeProps, FocusScopeEmits } from './types'
</script>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Primitive } from '../Primitive'
import { getActiveElement } from '../shared'
import { getFocusableElements, getFirstFocusable, getLastFocusable } from './utils'
import type { FocusScopeProps, FocusScopeEmits } from './types'

const { trapped = false, loop = false, as = 'div', asChild = false } = defineProps<FocusScopeProps>()

const emit = defineEmits<FocusScopeEmits>()

const initFocusContainer: HTMLElement | undefined = undefined
const containerRef = ref(initFocusContainer)
let previouslyFocusedElement: HTMLElement | null = null

function setContainerRef(el: HTMLElement | undefined) {
  if (el instanceof HTMLElement) {
    containerRef.value = el
  }
}

function handleKeyDown(event: KeyboardEvent) {
  if (!containerRef.value) return

  if (event.key !== 'Tab') return

  const focusableElements = getFocusableElements(containerRef.value)
  if (focusableElements.length === 0) return

  const first = focusableElements[0]!
  const last = focusableElements[focusableElements.length - 1]!
  const active = getActiveElement()

  if (trapped) {
    if (event.shiftKey && active === first) {
      if (loop) {
        event.preventDefault()
        last.focus()
      } else {
        event.preventDefault()
      }
    } else if (!event.shiftKey && active === last) {
      if (loop) {
        event.preventDefault()
        first.focus()
      } else {
        event.preventDefault()
      }
    }
  } else if (loop) {
    if (event.shiftKey && active === first) {
      event.preventDefault()
      last.focus()
    } else if (!event.shiftKey && active === last) {
      event.preventDefault()
      first.focus()
    }
  }
}

function focusTrap(event: FocusEvent) {
  if (!trapped || !containerRef.value) return

  const relatedTarget: HTMLElement | null = event.relatedTarget

  // If focus is moving outside the container, bring it back
  if (relatedTarget && !containerRef.value.contains(relatedTarget)) {
    const firstFocusable = getFirstFocusable(containerRef.value)
    firstFocusable?.focus()
  }
}

onMounted(() => {
  if (!containerRef.value) return

  previouslyFocusedElement = getActiveElement()

  const mountEvent = new CustomEvent('mountAutoFocus', { bubbles: false, cancelable: true })
  emit('mountAutoFocus', mountEvent)

  if (!mountEvent.defaultPrevented) {
    const firstFocusable = getFirstFocusable(containerRef.value)
    firstFocusable?.focus()
  }
})

onBeforeUnmount(() => {
  const unmountEvent = new CustomEvent('unmountAutoFocus', { bubbles: false, cancelable: true })
  emit('unmountAutoFocus', unmountEvent)

  if (!unmountEvent.defaultPrevented && previouslyFocusedElement) {
    previouslyFocusedElement.focus()
  }

  previouslyFocusedElement = null
})

watch(
  () => trapped,
  (isTapped) => {
    if (!containerRef.value) return

    if (isTapped) {
      containerRef.value.addEventListener('focusout', focusTrap)
    } else {
      containerRef.value.removeEventListener('focusout', focusTrap)
    }
  },
  { immediate: true },
)
</script>

<template>
  <Primitive
    :ref="setContainerRef"
    :as="as"
    :as-child="asChild"
    tabindex="-1"
    @keydown="handleKeyDown"
  >
    <slot />
  </Primitive>
</template>
