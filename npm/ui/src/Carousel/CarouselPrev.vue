<script lang="ts">
export type { CarouselPrevProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { injectCarouselRootContext } from './types'
import type { CarouselPrevProps } from './types'

const { as = 'button', asChild = false, disabled = false } = defineProps<CarouselPrevProps>()

const rootContext = injectCarouselRootContext('CarouselPrev')

const isDisabled = computed(() => disabled || !rootContext.canScrollPrev.value)

function handleClick() {
  if (isDisabled.value) return
  rootContext.scrollPrev()
}
</script>

<template>
  <Primitive
    :as="as || 'button'"
    :as-child="asChild"
    type="button"
    aria-label="Previous slide"
    :disabled="isDisabled || undefined"
    :data-disabled="isDisabled ? '' : undefined"
    data-vize-carousel-prev
    @click="handleClick"
  >
    <slot />
  </Primitive>
</template>
