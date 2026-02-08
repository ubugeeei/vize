<script lang="ts">
export type { PopoverContentSide, PopoverContentAlign, PopoverContentProps, PopoverContentEmits } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { Presence } from '../Presence'
import { DismissableLayer } from '../DismissableLayer'
import { FocusScope } from '../FocusScope'
import { injectPopoverRootContext } from './types'
import type { PopoverContentProps, PopoverContentEmits } from './types'

const {
  as = 'div',
  asChild = false,
  forceMount = false,
  side = 'bottom',
  sideOffset = 0,
  align = 'center',
  alignOffset = 0,
  trapFocus = false,
} = defineProps<PopoverContentProps>()

const emit = defineEmits<PopoverContentEmits>()

const context = injectPopoverRootContext('PopoverContent')

const present = computed(() => forceMount || context.open.value)

function handleDismiss() {
  context.onOpenChange(false)
}

function handleEscapeKeyDown(event: KeyboardEvent) {
  emit('escapeKeyDown', event)
}

function handlePointerDownOutside(event: PointerEvent) {
  emit('pointerDownOutside', event)
}

function handleFocusOutside(event: FocusEvent) {
  emit('focusOutside', event)
}

function handleInteractOutside(event: Event) {
  emit('interactOutside', event)
}

function handleMountAutoFocus(event: Event) {
  emit('openAutoFocus', event)
}

function handleUnmountAutoFocus(event: Event) {
  emit('closeAutoFocus', event)
}
</script>

<template>
  <Presence :present="present">
    <template #default="{ isPresent, ref: presenceRef, onAnimationStart, onAnimationEnd }">
      <FocusScope
        as-child
        :loop="trapFocus"
        :trapped="trapFocus"
        @mount-auto-focus="handleMountAutoFocus"
        @unmount-auto-focus="handleUnmountAutoFocus"
      >
        <DismissableLayer
          as-child
          :disable-outside-pointer-events="context.modal"
          @escape-key-down="handleEscapeKeyDown"
          @pointer-down-outside="handlePointerDownOutside"
          @focus-outside="handleFocusOutside"
          @interact-outside="handleInteractOutside"
          @dismiss="handleDismiss"
        >
          <Primitive
            :id="context.contentId"
            :ref="(el: any) => presenceRef(el?.$el ?? el)"
            :as="as"
            :as-child="asChild"
            role="dialog"
            :hidden="!isPresent ? true : undefined"
            :data-state="context.open.value ? 'open' : 'closed'"
            :data-side="side"
            :data-align="align"
            data-popover-content
            :style="{
              '--vize-popover-content-transform-origin': 'var(--vize-popover-content-transform-origin)',
              '--vize-popover-content-available-width': 'var(--vize-popover-content-available-width)',
              '--vize-popover-content-available-height': 'var(--vize-popover-content-available-height)',
              '--vize-popover-content-side-offset': `${sideOffset}px`,
              '--vize-popover-content-align-offset': `${alignOffset}px`,
            }"
            @animationstart="onAnimationStart"
            @animationend="onAnimationEnd"
          >
            <slot />
          </Primitive>
        </DismissableLayer>
      </FocusScope>
    </template>
  </Presence>
</template>
