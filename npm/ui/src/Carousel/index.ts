export { default as CarouselRoot } from './CarouselRoot.vue'
export { default as CarouselViewport } from './CarouselViewport.vue'
export { default as CarouselSlide } from './CarouselSlide.vue'
export { default as CarouselPrev } from './CarouselPrev.vue'
export { default as CarouselNext } from './CarouselNext.vue'
export { default as CarouselDots } from './CarouselDots.vue'
export { default as CarouselDot } from './CarouselDot.vue'
export type {
  CarouselRootProps,
  CarouselRootContext,
  CarouselViewportProps,
  CarouselSlideProps,
  CarouselPrevProps,
  CarouselNextProps,
  CarouselDotsProps,
  CarouselDotProps,
} from './types'
export {
  injectCarouselRootContext,
  provideCarouselRootContext,
} from './types'
