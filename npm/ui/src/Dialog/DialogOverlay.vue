<script lang="ts">
export type { DialogOverlayProps } from './types'
</script>

<script setup lang="ts">
import { computed, toRef } from 'vue'
import { Primitive } from '../Primitive'
import { usePresence } from '../Presence'
import { injectDialogRootContext } from './types'
import type { DialogOverlayProps } from './types'

const { as = 'div', asChild = false, forceMount = false } = defineProps<DialogOverlayProps>()

const context = injectDialogRootContext('DialogOverlay')

const present = computed(() => forceMount || context.open.value)
const { isPresent, ref: presenceRef, onAnimationStart, onAnimationEnd } = usePresence(toRef(present))

function handleRef(el: any) {
  presenceRef(el?.$el ?? el)
}
</script>

<template>
  <Primitive
    v-if="isPresent"
    :ref="handleRef"
    :as="as"
    :as-child="asChild"
    :hidden="!isPresent ? true : undefined"
    :data-state="context.open.value ? 'open' : 'closed'"
    data-dialog-overlay
    @animationstart="onAnimationStart"
    @animationend="onAnimationEnd"
  >
    <slot />
  </Primitive>
</template>
