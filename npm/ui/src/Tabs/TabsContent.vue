<script lang="ts">
export type { TabsContentProps } from './types'
</script>

<script setup lang="ts">
import { computed, toRef } from 'vue'
import { Primitive } from '../Primitive'
import { usePresence } from '../Presence'
import { injectTabsRootContext } from './types'
import type { TabsContentProps } from './types'

const { as = 'div', asChild = false, value, forceMount = false } = defineProps<TabsContentProps>()

const rootContext = injectTabsRootContext('TabsContent')

const triggerId = rootContext.baseId.concat('-trigger-', value)
const panelId = rootContext.baseId.concat('-panel-', value)
const isSelected = computed(() => rootContext.modelValue.value === value)
const present = computed(() => forceMount || isSelected.value)
const { isPresent, ref: presenceRef, onAnimationStart, onAnimationEnd } = usePresence(toRef(present))

function handleRef(el: any) {
  presenceRef(el?.$el ?? el)
}
</script>

<template>
  <Primitive
    v-if="isPresent"
    :id="panelId"
    :ref="handleRef"
    :as="as"
    :as-child="asChild"
    role="tabpanel"
    :aria-labelledby="triggerId"
    :tabindex="0"
    :hidden="!isPresent ? true : undefined"
    :data-state="isSelected ? 'active' : 'inactive'"
    :data-orientation="rootContext.orientation"
    data-vize-tabs-content
    @animationstart="onAnimationStart"
    @animationend="onAnimationEnd"
  >
    <slot />
  </Primitive>
</template>
