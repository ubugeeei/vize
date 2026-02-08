<script setup>
import { CarouselRoot, CarouselViewport, CarouselSlide, CarouselPrev, CarouselNext, CarouselDots, CarouselDot } from './index'
</script>

<art title="Carousel" component="./CarouselRoot.vue" category="Media" status="ready" tags="carousel,slider,gallery">
  <variant name="Default" default>
    <CarouselRoot>
      <CarouselViewport>
        <CarouselSlide>Slide 1</CarouselSlide>
        <CarouselSlide>Slide 2</CarouselSlide>
        <CarouselSlide>Slide 3</CarouselSlide>
      </CarouselViewport>
      <CarouselPrev />
      <CarouselNext />
    </CarouselRoot>
  </variant>

  <variant name="With Dots">
    <CarouselRoot>
      <CarouselViewport>
        <CarouselSlide>Slide 1</CarouselSlide>
        <CarouselSlide>Slide 2</CarouselSlide>
        <CarouselSlide>Slide 3</CarouselSlide>
      </CarouselViewport>
      <CarouselPrev />
      <CarouselNext />
      <CarouselDots v-slot="{ dots }">
        <CarouselDot v-for="dot in dots" :key="dot.index" :index="dot.index" />
      </CarouselDots>
    </CarouselRoot>
  </variant>

  <variant name="Loop">
    <CarouselRoot loop>
      <CarouselViewport>
        <CarouselSlide>Slide 1</CarouselSlide>
        <CarouselSlide>Slide 2</CarouselSlide>
        <CarouselSlide>Slide 3</CarouselSlide>
      </CarouselViewport>
      <CarouselPrev />
      <CarouselNext />
    </CarouselRoot>
  </variant>

  <variant name="Vertical">
    <CarouselRoot orientation="vertical">
      <CarouselViewport>
        <CarouselSlide>Slide 1</CarouselSlide>
        <CarouselSlide>Slide 2</CarouselSlide>
        <CarouselSlide>Slide 3</CarouselSlide>
      </CarouselViewport>
      <CarouselPrev />
      <CarouselNext />
    </CarouselRoot>
  </variant>
</art>
