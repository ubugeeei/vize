<script lang="ts">
export type { DialogDescriptionProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import { injectDialogRootContext } from './types'
import type { DialogDescriptionProps } from './types'

const { as = 'p', asChild = false } = defineProps<DialogDescriptionProps>()

const context = injectDialogRootContext('DialogDescription')
</script>

<template>
  <Primitive
    :id="context.descriptionId"
    :as="as"
    :as-child="asChild"
  >
    <slot />
  </Primitive>
</template>
