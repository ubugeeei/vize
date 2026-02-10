<script lang="ts">
export type { CarouselViewportProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { injectCarouselRootContext } from './types'
import type { CarouselViewportProps } from './types'

const { as = 'div', asChild = false } = defineProps<CarouselViewportProps>()

const rootContext = injectCarouselRootContext('CarouselViewport')

const liveText = computed(() =>
  `Slide ${rootContext.currentIndex.value + 1} of ${rootContext.totalSlides.value}`,
)
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    :data-orientation="rootContext.orientation"
    data-vize-carousel-viewport
    aria-live="polite"
    style="overflow: hidden"
  >
    <slot />
  </Primitive>
</template>
