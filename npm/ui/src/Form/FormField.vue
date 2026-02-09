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
// Avoid template literals - Rust compiler hoists them outside setup()
const controlId = id.concat('-control')
const messageId = id.concat('-message')

const fieldErrors = computed(() => rootContext.errors.value.get(name) || [])
const isTouched = computed(() => rootContext.touched.value.has(name))
const isDirty = computed(() => rootContext.dirty.value.has(name))
const isInvalid = computed(() => fieldErrors.value.length > 0)
const value = computed(() => {
  const fieldName = String(name)
  return rootContext.values.value[fieldName]
})

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
