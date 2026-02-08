<script lang="ts">
export type { DialogPortalProps } from './types'
</script>

<script setup lang="ts">
import { Teleport } from 'vue'
import type { DialogPortalProps } from './types'

const { to = 'body', disabled = false } = defineProps<DialogPortalProps>()
</script>

<template>
  <Teleport :to="to" :disabled="disabled">
    <slot />
  </Teleport>
</template>
