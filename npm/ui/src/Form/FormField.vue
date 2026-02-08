<script lang="ts">
export type { FormFieldProps, FormFieldContext } from './types'
export { injectFormFieldContext, provideFormFieldContext } from './types'
</script>

<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount } from 'vue'
import { useId } from '../shared'
import type { FormFieldProps } from './types'
import { injectFormRootContext, provideFormFieldContext } from './types'

const { name, defaultValue } = defineProps<FormFieldProps>()

const rootContext = injectFormRootContext('FormField')

const id = useId()
const controlId = `${id}-control`
const messageId = `${id}-message`

const fieldErrors = computed(() => rootContext.errors.value.get(name) || [])
const isTouched = computed(() => rootContext.touched.value.has(name))
const isDirty = computed(() => rootContext.dirty.value.has(name))
const isInvalid = computed(() => fieldErrors.value.length > 0)
const value = computed(() => rootContext.values.value[name])

function setValue(val: unknown) {
  rootContext.setFieldValue(name, val)
}

function setTouched() {
  rootContext.setFieldTouched(name)
}

onMounted(() => {
  rootContext.registerField(name, defaultValue)
})

onBeforeUnmount(() => {
  rootContext.unregisterField(name)
})

provideFormFieldContext({
  name,
  id,
  controlId,
  messageId,
  errors: fieldErrors,
  isTouched,
  isDirty,
  isInvalid,
  value,
  setValue,
  setTouched,
})
</script>

<template>
  <slot
    :value="value"
    :errors="fieldErrors"
    :is-touched="isTouched"
    :is-dirty="isDirty"
    :is-invalid="isInvalid"
    :name="name"
    :id="id"
  />
</template>
