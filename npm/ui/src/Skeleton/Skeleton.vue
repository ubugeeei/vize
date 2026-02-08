<script lang="ts">
export type { SkeletonProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { SkeletonProps } from './types'

const {
  as = 'div',
  asChild = false,
  loading = true,
  animated = true,
  width,
  height,
  radius,
  circle = false,
} = defineProps<SkeletonProps>()

function toCssValue(value: string | number | undefined): string | undefined {
  if (value === undefined) return undefined
  return typeof value === 'number' ? `${value}px` : value
}

const skeletonStyle = computed(() => {
  if (!loading) return undefined

  const size = circle ? (width ?? height) : undefined

  return {
    width: toCssValue(circle ? size : width),
    height: toCssValue(circle ? size : height),
    borderRadius: circle ? '50%' : toCssValue(radius),
  }
})
</script>

<template>
  <Primitive
    v-if="loading"
    :as="as"
    :as-child="asChild"
    aria-busy="true"
    aria-hidden="true"
    data-skeleton
    :data-loading="loading ? '' : undefined"
    :data-animated="animated ? '' : undefined"
    :style="skeletonStyle"
  >
    <slot name="skeleton" />
  </Primitive>
  <slot v-else />
</template>
