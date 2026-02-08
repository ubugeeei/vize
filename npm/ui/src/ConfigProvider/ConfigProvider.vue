<script lang="ts">
export type { ConfigProviderProps, ConfigProviderContext } from './types'
</script>

<script setup lang="ts">
import { computed, provide, toRef } from 'vue'
import { provideDirection } from '../shared'
import type { ConfigProviderProps } from './types'

const {
  dir = 'ltr',
  useId,
  scrollBody = true,
  nonce,
} = defineProps<ConfigProviderProps>()

const CONFIG_PROVIDER_KEY = Symbol('ConfigProvider')

const dirRef = computed(() => dir)
provideDirection(dirRef)

provide(CONFIG_PROVIDER_KEY, {
  dir: computed(() => dir),
  useId,
  scrollBody: computed(() => scrollBody),
  nonce: computed(() => nonce),
})
</script>

<template>
  <slot />
</template>
