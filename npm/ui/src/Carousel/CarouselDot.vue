<script lang="ts">
export type { CarouselDotProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { injectCarouselRootContext } from './types'
import type { CarouselDotProps } from './types'

const { as = 'button', asChild = false, index } = defineProps<CarouselDotProps>()

const rootContext = injectCarouselRootContext('CarouselDot')

const isActive = computed(() => rootContext.currentIndex.value === index)

function handleClick() {
  rootContext.scrollTo(index)
}
</script>

<template>
  <Primitive
    :as="as || 'button'"
    :as-child="asChild"
    type="button"
    role="tab"
    :aria-selected="isActive"
    :aria-label="`Go to slide ${index + 1}`"
    :data-state="isActive ? 'active' : 'inactive'"
    data-vize-carousel-dot
    @click="handleClick"
  >
    <slot />
  </Primitive>
</template>
