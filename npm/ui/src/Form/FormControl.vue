<script lang="ts">
export type { FormControlProps } from './types'
</script>

<script setup lang="ts">
import { Primitive } from '../Primitive'
import type { FormControlProps } from './types'
import { injectFormFieldContext, injectFormRootContext } from './types'

const { as = 'input', asChild = false } = defineProps<FormControlProps>()

const fieldContext = injectFormFieldContext('FormControl')
const rootContext = injectFormRootContext('FormControl')

function handleBlur() {
  fieldContext.setTouched()
}
</script>

<template>
  <Primitive
    :as="as || 'input'"
    :as-child="asChild"
    :id="fieldContext.controlId"
    :name="fieldContext.name"
    :aria-describedby="fieldContext.messageId"
    :aria-invalid="fieldContext.isInvalid.value || undefined"
    :disabled="rootContext.disabled || undefined"
    :data-invalid="fieldContext.isInvalid.value ? '' : undefined"
    data-vize-form-control
    @blur="handleBlur"
  >
    <slot />
  </Primitive>
</template>
