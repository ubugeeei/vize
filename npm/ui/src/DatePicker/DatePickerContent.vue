<script lang="ts">
export type { DatePickerContentProps } from './types'
</script>

<script setup lang="ts">
import { computed, ref, onMounted, onBeforeUnmount, Teleport } from 'vue'
import { Primitive } from '../Primitive'
import { kbd } from '../shared'
import type { DatePickerContentProps } from './types'
import { injectDatePickerRootContext } from './types'

const { as = 'div', asChild = false, forceMount = false, to = 'body', disableTeleport = false } = defineProps<DatePickerContentProps>()

const context = injectDatePickerRootContext('DatePickerContent')

const contentRef = ref<HTMLElement>()
const present = computed(() => forceMount || context.open.value)

function handleEscapeKeyDown(event: KeyboardEvent) {
  if (event.key === kbd.ESCAPE) {
    context.onOpenChange(false)
    context.triggerRef.value?.focus()
  }
}

function handlePointerDownOutside(event: PointerEvent) {
  const target: HTMLElement = event.target
  // Ignore clicks on the trigger
  if (context.triggerRef.value?.contains(target)) return
  // Ignore clicks inside the content
  if (contentRef.value?.contains(target)) return

  context.onOpenChange(false)
}

function setContentRef(el: any) {
  contentRef.value = el?.$el ?? el
}

// Set up outside click listener
onMounted(() => {
  document.addEventListener('pointerdown', handlePointerDownOutside)
  document.addEventListener('keydown', handleEscapeKeyDown)
})

onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', handlePointerDownOutside)
  document.removeEventListener('keydown', handleEscapeKeyDown)
})
</script>

<template>
  <Teleport :to="to" :disabled="disableTeleport">
    <Primitive
      v-if="present"
      :id="context.contentId"
      :ref="setContentRef"
      :as="as"
      :as-child="asChild"
      role="dialog"
      aria-modal="true"
      aria-label="Calendar"
      :data-state="context.open.value ? 'open' : 'closed'"
      data-vize-datepicker-content
    >
      <slot />
    </Primitive>
  </Teleport>
</template>
