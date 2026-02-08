<script lang="ts">
export type { TooltipPortalProps } from './types'
</script>

<script setup lang="ts">
import { Teleport } from 'vue'
import type { TooltipPortalProps } from './types'

const { to = 'body', disabled = false } = defineProps<TooltipPortalProps>()
</script>

<template>
  <Teleport :to="to" :disabled="disabled">
    <slot />
  </Teleport>
</template>
