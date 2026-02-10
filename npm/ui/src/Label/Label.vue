<script lang="ts">
export type { LabelProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { LabelProps } from './types'

const { as = 'label', asChild = false, for: htmlFor } = defineProps<LabelProps>()
</script>

<template>
  <Primitive :as="as || 'label'" :as-child="asChild" :for="htmlFor" data-vize-label @mousedown.prevent>
    <slot />
  </Primitive>
</template>
