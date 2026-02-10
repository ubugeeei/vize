<script lang="ts">
export type { CarouselRootProps, CarouselRootContext } from './types'
export { injectCarouselRootContext, provideCarouselRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { Primitive } from '../Primitive'
import { useDirection, kbd } from '../shared'
import type { CarouselRootProps } from './types'
import { provideCarouselRootContext } from './types'

const {
  as = 'div',
  asChild = false,
  orientation = 'horizontal',
  loop = false,
  dir: dirProp,
  defaultValue = 0,
  modelValue,
  autoplay = 0,
  slidesPerView = 1,
} = defineProps<CarouselRootProps>()

const emit = defineEmits<{
  'update:modelValue': [value: number]
}>()

const direction = useDirection(computed(() => dirProp))

const internal = ref(defaultValue)
const currentIndex = computed(() => modelValue !== undefined ? modelValue : internal.value)

function updateValue(val: number) {
  internal.value = val
  emit('update:modelValue', val)
}

const totalSlides = ref(0)
let slideCounter = 0

function registerSlide(): number {
  const index = slideCounter++
  totalSlides.value = slideCounter
  return index
}

const maxIndex = computed(() => Math.max(0, totalSlides.value - slidesPerView))

const canScrollPrev = computed(() => loop || currentIndex.value > 0)
const canScrollNext = computed(() => loop || currentIndex.value < maxIndex.value)

function scrollTo(index: number) {
  let target = index
  if (loop) {
    if (target < 0) target = maxIndex.value
    else if (target > maxIndex.value) target = 0
  } else {
    target = Math.max(0, Math.min(target, maxIndex.value))
  }
  updateValue(target)
}

function scrollPrev() {
  scrollTo(currentIndex.value - 1)
}

function scrollNext() {
  scrollTo(currentIndex.value + 1)
}

// Autoplay
let autoplayTimer: ReturnType<typeof setInterval> | undefined

function startAutoplay() {
  if (autoplay > 0) {
    autoplayTimer = setInterval(() => {
      scrollNext()
    }, autoplay)
  }
}

function stopAutoplay() {
  if (autoplayTimer !== undefined) {
    clearInterval(autoplayTimer)
    autoplayTimer = undefined
  }
}

onMounted(() => {
  startAutoplay()
})

onUnmounted(() => {
  stopAutoplay()
})

watch(() => autoplay, () => {
  stopAutoplay()
  startAutoplay()
})

function handleKeydown(event: KeyboardEvent) {
  const isHorizontal = orientation === 'horizontal'
  const isRtl = direction.value === 'rtl'

  let handled = false

  if (isHorizontal) {
    if (event.key === kbd.ARROW_LEFT) {
      if (isRtl) { scrollNext() } else { scrollPrev() }
      handled = true
    } else if (event.key === kbd.ARROW_RIGHT) {
      if (isRtl) { scrollPrev() } else { scrollNext() }
      handled = true
    }
  } else {
    if (event.key === kbd.ARROW_UP) {
      scrollPrev()
      handled = true
    } else if (event.key === kbd.ARROW_DOWN) {
      scrollNext()
      handled = true
    }
  }

  if (event.key === kbd.HOME) {
    scrollTo(0)
    handled = true
  } else if (event.key === kbd.END) {
    scrollTo(maxIndex.value)
    handled = true
  }

  if (handled) {
    event.preventDefault()
  }
}

provideCarouselRootContext({
  currentIndex,
  totalSlides,
  orientation,
  loop,
  dir: direction,
  canScrollPrev,
  canScrollNext,
  scrollTo,
  scrollPrev,
  scrollNext,
  registerSlide,
  slidesPerView,
})
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    role="region"
    aria-roledescription="carousel"
    :data-orientation="orientation"
    data-vize-carousel
    :dir="direction"
    @keydown="handleKeydown"
  >
    <slot />
  </Primitive>
</template>
