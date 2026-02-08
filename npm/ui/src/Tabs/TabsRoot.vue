<script lang="ts">
export type { ActivationMode, TabsRootProps, TabsRootContext } from './types'
export { injectTabsRootContext, provideTabsRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Primitive } from '../Primitive'
import { useDirection } from '../shared'
import type { TabsRootProps } from './types'
import { provideTabsRootContext } from './types'

const {
  as = 'div',
  asChild = false,
  orientation = 'horizontal',
  dir: dirProp,
  activationMode = 'automatic',
  defaultValue = '',
  modelValue,
} = defineProps<TabsRootProps>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const direction = useDirection(computed(() => dirProp))

const internal = ref(defaultValue)
const currentValue = computed(() => modelValue !== undefined ? modelValue : internal.value)

function changeValue(value: string) {
  internal.value = value
  emit('update:modelValue', value)
}

const parentRef = ref<HTMLElement>()

provideTabsRootContext({
  modelValue: currentValue,
  orientation,
  dir: direction,
  activationMode,
  changeValue,
  parentRef,
})
</script>

<template>
  <Primitive
    :ref="(el) => { parentRef = el?.$el ?? el }"
    :as="as"
    :as-child="asChild"
    :data-orientation="orientation"
  >
    <slot />
  </Primitive>
</template>
