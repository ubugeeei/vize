<script lang="ts">
export type { DismissableLayerProps, DismissableLayerEmits } from './types'
</script>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { Primitive } from '../Primitive'
import { kbd } from '../shared'
import {
  type LayerEntry,
  layerStack,
  isPointerDownOutside,
  isFocusOutside,
  isTopLayer,
} from './utils'
import type { DismissableLayerProps, DismissableLayerEmits } from './types'

const {
  disableOutsidePointerEvents = false,
  as = 'div',
  asChild = false,
} = defineProps<DismissableLayerProps>()

const emit = defineEmits<DismissableLayerEmits>()

const layerRef = ref<HTMLElement>()
let layerEntry: LayerEntry | undefined

function setLayerRef(el: HTMLElement | undefined) {
  if (el instanceof HTMLElement) {
    layerRef.value = el
  }
}

function handleKeyDown(event: KeyboardEvent) {
  if (event.key !== kbd.ESCAPE) return
  if (!layerRef.value || !isTopLayer(layerRef.value)) return

  emit('escapeKeyDown', event)
  if (!event.defaultPrevented) {
    emit('dismiss')
  }
}

function handlePointerDown(event: PointerEvent) {
  if (!layerRef.value) return
  if (!isTopLayer(layerRef.value)) return
  if (!isPointerDownOutside(event, layerRef.value)) return

  emit('pointerDownOutside', event)
  emit('interactOutside', event)
  if (!event.defaultPrevented) {
    emit('dismiss')
  }
}

function handleFocusOut(event: FocusEvent) {
  if (!layerRef.value) return
  if (!isTopLayer(layerRef.value)) return
  if (!isFocusOutside(event, layerRef.value)) return

  emit('focusOutside', event)
  emit('interactOutside', event)
  if (!event.defaultPrevented) {
    emit('dismiss')
  }
}

onMounted(() => {
  if (!layerRef.value) return

  layerEntry = {
    element: layerRef.value,
    disableOutsidePointerEvents,
  }
  layerStack.add(layerEntry)

  if (disableOutsidePointerEvents) {
    document.body.style.pointerEvents = 'none'
    layerRef.value.style.pointerEvents = 'auto'
  }

  document.addEventListener('keydown', handleKeyDown)
  document.addEventListener('pointerdown', handlePointerDown, true)
})

onBeforeUnmount(() => {
  if (layerEntry) {
    layerStack.delete(layerEntry)
  }

  // Restore pointer events if no more layers with disabled pointer events
  const hasDisabled = [...layerStack].some(
    (entry) => entry.disableOutsidePointerEvents,
  )
  if (!hasDisabled) {
    document.body.style.pointerEvents = ''
  }

  document.removeEventListener('keydown', handleKeyDown)
  document.removeEventListener('pointerdown', handlePointerDown, true)
})
</script>

<template>
  <Primitive
    :ref="setLayerRef"
    :as="as"
    :as-child="asChild"
    data-dismissable-layer
    @focusout="handleFocusOut"
  >
    <slot />
  </Primitive>
</template>
