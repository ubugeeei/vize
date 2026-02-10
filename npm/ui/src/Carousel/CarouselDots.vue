<script lang="ts">
export type { CarouselDotsProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { injectCarouselRootContext } from './types'
import type { CarouselDotsProps } from './types'

const { as = 'div', asChild = false } = defineProps<CarouselDotsProps>()

const rootContext = injectCarouselRootContext('CarouselDots')

const dots = computed(() =>
  Array.from({ length: rootContext.totalSlides.value }, (_, i) => ({
    index: i,
    isActive: i === rootContext.currentIndex.value,
  })),
)
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="tablist"
    data-vize-carousel-dots
  >
    <slot :dots="dots" />
  </Primitive>
</template>
