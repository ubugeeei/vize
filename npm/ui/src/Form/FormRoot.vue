<script lang="ts">
export type { FormRootProps, FormRootContext, FieldError } from './types'
export { injectFormRootContext, provideFormRootContext } from './types'
</script>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Primitive } from '../Primitive'
import type { FormRootProps, FieldError, StandardSchemaV1Issue } from './types'
import { provideFormRootContext } from './types'

const {
  as = 'form',
  asChild = false,
  schema,
  validationMode = 'onSubmit',
  disabled = false,
} = defineProps<FormRootProps>()

const emit = defineEmits<{
  'submit': [values: Record<string, unknown>]
  'submitInvalid': [errors: FieldError[]]
}>()

const errors = ref<Map<string, string[]>>(new Map())
const touched = ref<Set<string>>(new Set())
const dirty = ref<Set<string>>(new Set())
const isSubmitting = ref(false)
const values = ref<Record<string, unknown>>({})
const defaultValues = ref<Record<string, unknown>>({})

const isValid = computed(() => {
  for (const fieldErrors of errors.value.values()) {
    if (fieldErrors.length > 0) return false
  }
  return true
})

function setFieldValue(name: string, value: unknown) {
  values.value = { ...values.value, [name]: value }
  dirty.value = new Set([...dirty.value, name])

  if (validationMode === 'onChange') {
    validateField(name)
  }
  if (validationMode === 'onTouched' && touched.value.has(name)) {
    validateField(name)
  }
}

function setFieldTouched(name: string) {
  touched.value = new Set([...touched.value, name])

  if (validationMode === 'onBlur' || validationMode === 'onTouched') {
    validateField(name)
  }
}

function setFieldError(name: string, fieldErrors: string[]) {
  const next = new Map(errors.value)
  next.set(name, fieldErrors)
  errors.value = next
}

function clearFieldError(name: string) {
  const next = new Map(errors.value)
  next.delete(name)
  errors.value = next
}

function issuesToFieldPath(issue: StandardSchemaV1Issue): string {
  if (!issue.path || issue.path.length === 0) return ''
  return issue.path.map(String).join('.')
}

async function validateField(name: string): Promise<void> {
  if (!schema) {
    clearFieldError(name)
    return
  }

  const result = await schema['~standard'].validate(values.value)
  if (result.issues) {
    const fieldIssues = result.issues.filter(
      issue => issuesToFieldPath(issue) === name,
    )
    if (fieldIssues.length > 0) {
      setFieldError(name, fieldIssues.map(i => i.message))
    } else {
      clearFieldError(name)
    }
  } else {
    clearFieldError(name)
  }
}

async function validate(): Promise<boolean> {
  if (!schema) return true

  const result = await schema['~standard'].validate(values.value)
  if (result.issues) {
    const next = new Map<string, string[]>()
    for (const issue of result.issues) {
      const path = issuesToFieldPath(issue)
      if (!path) continue
      const existing = next.get(path) || []
      existing.push(issue.message)
      next.set(path, existing)
    }
    errors.value = next
    return false
  }

  errors.value = new Map()
  return true
}

function reset() {
  errors.value = new Map()
  touched.value = new Set()
  dirty.value = new Set()
  values.value = { ...defaultValues.value }
}

function registerField(name: string, defaultValue?: unknown) {
  if (defaultValue !== undefined) {
    defaultValues.value = { ...defaultValues.value, [name]: defaultValue }
    if (!(name in values.value)) {
      values.value = { ...values.value, [name]: defaultValue }
    }
  }
}

function unregisterField(name: string) {
  const nextValues = { ...values.value }
  delete nextValues[name]
  values.value = nextValues

  const nextDefaults = { ...defaultValues.value }
  delete nextDefaults[name]
  defaultValues.value = nextDefaults

  clearFieldError(name)

  const nextTouched = new Set(touched.value)
  nextTouched.delete(name)
  touched.value = nextTouched

  const nextDirty = new Set(dirty.value)
  nextDirty.delete(name)
  dirty.value = nextDirty
}

async function handleSubmit(event: Event) {
  event.preventDefault()
  if (disabled) return

  isSubmitting.value = true
  try {
    const valid = await validate()
    if (valid) {
      emit('submit', { ...values.value })
    } else {
      const fieldErrors: FieldError[] = []
      for (const [path, messages] of errors.value.entries()) {
        for (const message of messages) {
          fieldErrors.push({ path, message })
        }
      }
      emit('submitInvalid', fieldErrors)
    }
  } finally {
    isSubmitting.value = false
  }
}

provideFormRootContext({
  disabled,
  validationMode,
  errors,
  touched,
  dirty,
  isSubmitting,
  isValid,
  values,
  setFieldValue,
  setFieldTouched,
  setFieldError,
  clearFieldError,
  validateField,
  validate,
  reset,
  registerField,
  unregisterField,
})
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    :data-submitting="isSubmitting ? '' : undefined"
    :data-valid="isValid ? '' : undefined"
    :data-invalid="!isValid ? '' : undefined"
    data-vize-form
    @submit="handleSubmit"
  >
    <slot />
  </Primitive>
</template>
