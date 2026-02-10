<script lang="ts">
export type { CarouselSlideProps } from './types'
</script>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Primitive } from '../Primitive'
import { injectCarouselRootContext } from './types'
import type { CarouselSlideProps } from './types'

const { as = 'div', asChild = false } = defineProps<CarouselSlideProps>()

const rootContext = injectCarouselRootContext('CarouselSlide')

const slideIndex = ref(-1)

onMounted(() => {
  slideIndex.value = rootContext.registerSlide()
})

const isActive = computed(() => {
  const current = rootContext.currentIndex.value
  const perView = rootContext.slidesPerView
  return slideIndex.value >= current && slideIndex.value < current + perView
})

const label = computed(() =>
  `${slideIndex.value + 1} of ${rootContext.totalSlides.value}`,
)
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="group"
    aria-roledescription="slide"
    :aria-label="label"
    :data-active="isActive ? '' : undefined"
    :data-index="slideIndex"
    data-vize-carousel-slide
  >
    <slot />
  </Primitive>
</template>
