<script lang="ts">
export type { CarouselNextProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { injectCarouselRootContext } from './types'
import type { CarouselNextProps } from './types'

const { as = 'button', asChild = false, disabled = false } = defineProps<CarouselNextProps>()

const rootContext = injectCarouselRootContext('CarouselNext')

const isDisabled = computed(() => disabled || !rootContext.canScrollNext.value)

function handleClick() {
  if (isDisabled.value) return
  rootContext.scrollNext()
}
</script>

<template>
  <Primitive
    :as="as || 'button'"
    :as-child="asChild"
    type="button"
    aria-label="Next slide"
    :disabled="isDisabled || undefined"
    :data-disabled="isDisabled ? '' : undefined"
    data-vize-carousel-next
    @click="handleClick"
  >
    <slot />
  </Primitive>
</template>
