<script lang="ts">
export type { SwitchThumbProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import { injectSwitchRootContext } from './types'
import type { SwitchThumbProps } from './types'

const { as = 'span', asChild = false } = defineProps<SwitchThumbProps>()

const context = injectSwitchRootContext('SwitchThumb')
</script>

<template>
  <Primitive
    :as="as || 'span'"
    :as-child="asChild"
    :data-state="context.checked.value ? 'checked' : 'unchecked'"
    :data-disabled="context.disabled ? '' : undefined"
    data-vize-switch-thumb
  >
    <slot />
  </Primitive>
</template>
