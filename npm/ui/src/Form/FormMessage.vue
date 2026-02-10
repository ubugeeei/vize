<script lang="ts">
export type { FormMessageProps } from './types'
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { Primitive } from '../Primitive'
import type { FormMessageProps } from './types'
import { injectFormFieldContext } from './types'

const { as = 'span', asChild = false, match } = defineProps<FormMessageProps>()

const fieldContext = injectFormFieldContext('FormMessage')

const displayMessage = computed<string | undefined>(() => {
  const errs = fieldContext.errors.value
  if (errs.length === 0) return undefined

  if (match === undefined) {
    return errs[0]
  }
  if (typeof match === 'string') {
    return errs.find(e => e === match)
  }
  return match(errs)
})

const hasError = computed(() => displayMessage.value !== undefined)
</script>

<template>
  <Primitive
    v-if="hasError"
    :as="as || 'span'"
    :as-child="asChild"
    :id="fieldContext.messageId"
    role="alert"
    aria-live="polite"
    :data-invalid="hasError ? '' : undefined"
    data-vize-form-message
  >
    <slot :message="displayMessage">
      {{ displayMessage }}
    </slot>
  </Primitive>
  <slot v-else />
</template>
