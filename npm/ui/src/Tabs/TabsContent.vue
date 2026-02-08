<script lang="ts">
export type { TabsContentProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import { Presence } from '../Presence'
import { useId } from '../shared'
import { injectTabsRootContext } from './types'
import type { TabsContentProps } from './types'

const { as = 'div', asChild = false, value, forceMount = false } = defineProps<TabsContentProps>()

const rootContext = injectTabsRootContext('TabsContent')

const triggerId = useId()
const isSelected = computed(() => rootContext.modelValue.value === value)
const present = computed(() => forceMount || isSelected.value)
</script>

<template>
  <Presence :present="present">
    <template #default="{ isPresent: presenceIsPresent, ref: presenceRef, onAnimationStart, onAnimationEnd }">
      <Primitive
        :id="`${triggerId}-panel-${value}`"
        :ref="(el: any) => { presenceRef(el?.$el ?? el) }"
        :as="as"
        :as-child="asChild"
        role="tabpanel"
        :aria-labelledby="triggerId"
        :tabindex="0"
        :hidden="!presenceIsPresent ? true : undefined"
        :data-state="isSelected ? 'active' : 'inactive'"
        :data-orientation="rootContext.orientation"
        @animationstart="onAnimationStart"
        @animationend="onAnimationEnd"
      >
        <slot />
      </Primitive>
    </template>
  </Presence>
</template>
