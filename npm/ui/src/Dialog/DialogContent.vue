<script lang="ts">
export type { DialogContentProps, DialogContentEmits } from './types'
</script>

<script setup lang="ts">
import { computed, toRef } from 'vue'
import { Primitive } from '../Primitive'
import { usePresence } from '../Presence'
import { FocusScope } from '../FocusScope'
import { DismissableLayer } from '../DismissableLayer'
import { useBodyScrollLock } from '../shared'
import { injectDialogRootContext } from './types'
import type { DialogContentProps, DialogContentEmits } from './types'

const {
  as = 'div',
  asChild = false,
  forceMount = false,
  trapFocus = true,
} = defineProps<DialogContentProps>()

const emit = defineEmits<DialogContentEmits>()

const context = injectDialogRootContext('DialogContent')

const present = computed(() => forceMount || context.open.value)
const { isPresent, ref: presenceRef, onAnimationStart, onAnimationEnd } = usePresence(toRef(present))

useBodyScrollLock(computed(() => context.open.value))

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
  <FocusScope
    v-if="isPresent"
    as-child
    :loop="trapFocus"
    :trapped="trapFocus"
    @mount-auto-focus="handleMountAutoFocus"
    @unmount-auto-focus="handleUnmountAutoFocus"
  >
    <DismissableLayer
      as-child
      disable-outside-pointer-events
      @escape-key-down="handleEscapeKeyDown"
      @pointer-down-outside="handlePointerDownOutside"
      @focus-outside="handleFocusOutside"
      @interact-outside="handleInteractOutside"
      @dismiss="handleDismiss"
    >
      <Primitive
        :id="context.contentId"
        :ref="handleRef"
        :as="as"
        :as-child="asChild"
        role="dialog"
        aria-modal="true"
        :aria-describedby="context.descriptionId"
        :aria-labelledby="context.titleId"
        :hidden="!isPresent ? true : undefined"
        :data-state="context.open.value ? 'open' : 'closed'"
        data-dialog-content
        @animationstart="onAnimationStart"
        @animationend="onAnimationEnd"
      >
        <slot />
      </Primitive>
    </DismissableLayer>
  </FocusScope>
</template>
