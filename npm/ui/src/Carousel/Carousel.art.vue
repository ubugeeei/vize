<script setup>
import { Carousel } from '../index'
</script>

<art title="Carousel" component="./CarouselRoot.vue" category="Media" status="ready" tags="carousel,slider,gallery">
  <variant name="Default" default>
    <Carousel.Root>
      <Carousel.Viewport>
        <Carousel.Slide>Slide 1</Carousel.Slide>
        <Carousel.Slide>Slide 2</Carousel.Slide>
        <Carousel.Slide>Slide 3</Carousel.Slide>
      </Carousel.Viewport>
      <Carousel.Prev />
      <Carousel.Next />
    </Carousel.Root>
  </variant>

  <variant name="With Dots">
    <Carousel.Root>
      <Carousel.Viewport>
        <Carousel.Slide>Slide 1</Carousel.Slide>
        <Carousel.Slide>Slide 2</Carousel.Slide>
        <Carousel.Slide>Slide 3</Carousel.Slide>
      </Carousel.Viewport>
      <Carousel.Prev />
      <Carousel.Next />
      <Carousel.Dots v-slot="{ dots }">
        <Carousel.Dot v-for="dot in dots" :key="dot.index" :index="dot.index" />
      </Carousel.Dots>
    </Carousel.Root>
  </variant>

  <variant name="Loop">
    <Carousel.Root loop>
      <Carousel.Viewport>
        <Carousel.Slide>Slide 1</Carousel.Slide>
        <Carousel.Slide>Slide 2</Carousel.Slide>
        <Carousel.Slide>Slide 3</Carousel.Slide>
      </Carousel.Viewport>
      <Carousel.Prev />
      <Carousel.Next />
    </Carousel.Root>
  </variant>

  <variant name="Vertical">
    <Carousel.Root orientation="vertical">
      <Carousel.Viewport>
        <Carousel.Slide>Slide 1</Carousel.Slide>
        <Carousel.Slide>Slide 2</Carousel.Slide>
        <Carousel.Slide>Slide 3</Carousel.Slide>
      </Carousel.Viewport>
      <Carousel.Prev />
      <Carousel.Next />
    </Carousel.Root>
  </variant>
</art>
