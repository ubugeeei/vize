<script lang="ts">
export type { FormSubmitProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { FormSubmitProps } from './types'
import { injectFormRootContext } from './types'

const { as = 'button', asChild = false, disabled = false } = defineProps<FormSubmitProps>()

const rootContext = injectFormRootContext('FormSubmit')

const isDisabled = computed(() => disabled || rootContext.isSubmitting.value)
</script>

<template>
  <Primitive
    :as="as || 'button'"
    :as-child="asChild"
    type="submit"
    :disabled="isDisabled || undefined"
    :data-submitting="rootContext.isSubmitting.value ? '' : undefined"
    :data-disabled="isDisabled ? '' : undefined"
    data-vize-form-submit
  >
    <slot />
  </Primitive>
</template>
