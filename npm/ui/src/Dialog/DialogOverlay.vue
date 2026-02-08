<script lang="ts">
export type { DialogOverlayProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { Presence } from '../Presence'
import { injectDialogRootContext } from './types'
import type { DialogOverlayProps } from './types'

const { as = 'div', asChild = false, forceMount = false } = defineProps<DialogOverlayProps>()

const context = injectDialogRootContext('DialogOverlay')

const present = computed(() => forceMount || context.open.value)
</script>

<template>
  <Presence :present="present">
    <template #default="{ isPresent, ref: presenceRef, onAnimationStart, onAnimationEnd }">
      <Primitive
        :ref="(el: any) => presenceRef(el?.$el ?? el)"
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
  </Presence>
</template>
