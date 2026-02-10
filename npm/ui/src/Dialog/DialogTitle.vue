<script lang="ts">
export type { DialogTitleProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import { injectDialogRootContext } from './types'
import type { DialogTitleProps } from './types'

const { as = 'h2', asChild = false } = defineProps<DialogTitleProps>()

const context = injectDialogRootContext('DialogTitle')
</script>

<template>
  <Primitive
    :id="context.titleId"
    :as="as || 'h2'"
    :as-child="asChild"
    data-vize-dialog-title
  >
    <slot />
  </Primitive>
</template>
