<script lang="ts">
export type { SkeletonProps } from './types'
</script>

<script setup lang="ts">
import { computed, useAttrs } from 'vue'
import { Primitive } from '../Primitive'
import type { SkeletonProps } from './types'

// Rust compiler cannot resolve imported types (SkeletonProps is from ./types),
// so no runtime `props` option is generated in defineComponent.
// All passed props end up in $attrs. We use useAttrs() to access them.
defineProps<SkeletonProps>()
const attrs = useAttrs()

// Helper: attrs use kebab-case when props are not declared
function attr(name: string): unknown {
  if (name in attrs) return attrs[name]
  const kebab = name.replace(/[A-Z]/g, (m: string) => '-' + m.toLowerCase())
  return attrs[kebab]
}

const componentAs = computed(() => (attr('as') ?? 'div') as string)
const componentAsChild = computed(() => !!attr('asChild'))
const isLoading = computed(() => {
  const v = attr('loading')
  return v === undefined ? true : !!v
})
const isAnimated = computed(() => {
  const v = attr('animated')
  return v === undefined ? true : !!v
})
const isCircle = computed(() => !!attr('circle'))

function toCssValue(value: string | number | undefined): string | undefined {
  if (value === undefined) return undefined
  return typeof value === 'number' ? String(value) + 'px' : value
}

const skeletonStyle = computed(() => {
  if (!isLoading.value) return undefined

  const w = attr('width') as string | number | undefined
  const h = attr('height') as string | number | undefined
  const r = attr('radius') as string | number | undefined
  const size = isCircle.value ? (w ?? h) : undefined

  return {
    width: toCssValue(isCircle.value ? size : w),
    height: toCssValue(isCircle.value ? size : h),
    borderRadius: isCircle.value ? '50%' : toCssValue(r),
  }
})
</script>

<template>
  <Primitive
    v-if="isLoading"
    :as="componentAs"
    :as-child="componentAsChild"
    aria-busy="true"
    aria-hidden="true"
    data-vize-skeleton
    :data-loading="isLoading ? '' : undefined"
    :data-animated="isAnimated ? '' : undefined"
    :style="skeletonStyle"
  >
    <slot name="skeleton" />
  </Primitive>
  <slot v-else />
</template>
