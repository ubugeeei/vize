<script lang="ts">
export type { FormLabelProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { FormLabelProps } from './types'
import { injectFormFieldContext } from './types'

const { as = 'label', asChild = false } = defineProps<FormLabelProps>()

const fieldContext = injectFormFieldContext('FormLabel')
</script>

<template>
  <Primitive
    :as="as || 'label'"
    :as-child="asChild"
    :for="fieldContext.controlId"
    :data-invalid="fieldContext.isInvalid.value ? '' : undefined"
    data-vize-form-label
  >
    <slot />
  </Primitive>
</template>
