import type { ComputedRef, Ref } from 'vue'
import type { PrimitiveProps } from '../Primitive'
import type { Direction, Orientation } from '../shared'
import { createContext } from '../shared'

export interface CarouselRootProps extends PrimitiveProps {
  modelValue?: number
  defaultValue?: number
  orientation?: Orientation
  loop?: boolean
  dir?: Direction
  autoplay?: number
  slidesPerView?: number
}

export interface CarouselRootContext {
  currentIndex: ComputedRef<number>
  totalSlides: Ref<number>
  orientation: Orientation
  loop: boolean
  dir: ComputedRef<Direction>
  canScrollPrev: ComputedRef<boolean>
  canScrollNext: ComputedRef<boolean>
  scrollTo: (index: number) => void
  scrollPrev: () => void
  scrollNext: () => void
  registerSlide: () => number
  slidesPerView: number
}

const _carouselRootContext = createContext<CarouselRootContext>('CarouselRoot')
export const injectCarouselRootContext = _carouselRootContext[0]
export const provideCarouselRootContext = _carouselRootContext[1]

export interface CarouselViewportProps extends PrimitiveProps {}

export interface CarouselSlideProps extends PrimitiveProps {}

export interface CarouselPrevProps extends PrimitiveProps {
  disabled?: boolean
}

export interface CarouselNextProps extends PrimitiveProps {
  disabled?: boolean
}

export interface CarouselDotsProps extends PrimitiveProps {}

export interface CarouselDotProps extends PrimitiveProps {
  index: number
}
