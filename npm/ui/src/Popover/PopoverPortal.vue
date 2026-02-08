<script lang="ts">
export type { PopoverPortalProps } from './types'
</script>

<script setup lang="ts">
import { Teleport } from 'vue'
import type { PopoverPortalProps } from './types'

const { to = 'body', disabled = false } = defineProps<PopoverPortalProps>()
</script>

<template>
  <Teleport :to="to" :disabled="disabled">
    <slot />
  </Teleport>
</template>
