<script lang="ts">
export type { TooltipContentSide, TooltipContentAlign, TooltipContentProps, TooltipContentEmits } from './types'
</script>

<script setup lang="ts">
import { computed, toRef } from 'vue'
import { Primitive } from '../Primitive'
import { usePresence } from '../Presence'
import { DismissableLayer } from '../DismissableLayer'
import { injectTooltipRootContext } from './types'
import { injectTooltipProviderContext } from './types'
import type { TooltipContentProps, TooltipContentEmits } from './types'

const {
  as = 'div',
  asChild = false,
  forceMount = false,
  side = 'top',
  sideOffset = 0,
  align = 'center',
  alignOffset = 0,
} = defineProps<TooltipContentProps>()

const emit = defineEmits<TooltipContentEmits>()

const context = injectTooltipRootContext('TooltipContent')
const provider = injectTooltipProviderContext('TooltipContent')

const present = computed(() => forceMount || context.open.value)
const { isPresent, ref: presenceRef, onAnimationStart, onAnimationEnd } = usePresence(toRef(present))

function handleRef(el: any) {
  presenceRef(el?.$el ?? el)
}

function handleDismiss() {
  context.onOpenChange(false)
}

function handleEscapeKeyDown(event: KeyboardEvent) {
  emit('escapeKeyDown', event)
}

function handlePointerDownOutside(event: PointerEvent) {
  emit('pointerDownOutside', event)
}

function handlePointerEnter() {
  if (!provider.disableHoverableContent) {
    // Keep tooltip open when hovering content
  }
}

function handlePointerLeave() {
  context.onOpenChange(false)
}
</script>

<template>
  <DismissableLayer
    v-if="isPresent"
    as-child
    @escape-key-down="handleEscapeKeyDown"
    @pointer-down-outside="handlePointerDownOutside"
    @dismiss="handleDismiss"
  >
    <Primitive
      :id="context.contentId"
      :ref="handleRef"
      :as="as"
      :as-child="asChild"
      role="tooltip"
      :hidden="!isPresent ? true : undefined"
      :data-state="context.open.value ? 'open' : 'closed'"
      :data-side="side"
      :data-align="align"
      data-vize-tooltip-content
      :style="{
        '--vize-tooltip-content-transform-origin': 'var(--vize-tooltip-content-transform-origin)',
        '--vize-tooltip-content-available-width': 'var(--vize-tooltip-content-available-width)',
        '--vize-tooltip-content-available-height': 'var(--vize-tooltip-content-available-height)',
        '--vize-tooltip-content-side-offset': `${sideOffset}px`,
        '--vize-tooltip-content-align-offset': `${alignOffset}px`,
      }"
      @pointerenter="handlePointerEnter"
      @pointerleave="handlePointerLeave"
      @animationstart="onAnimationStart"
      @animationend="onAnimationEnd"
    >
      <slot />
    </Primitive>
  </DismissableLayer>
</template>
