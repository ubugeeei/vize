import type { ComputedRef, Ref } from 'vue'
import type { PrimitiveProps } from '../Primitive'
import { createContext } from '../shared'

// Standard Schema V1 types (inline, no dependency)
export interface StandardSchemaV1<Input = unknown, Output = Input> {
  readonly '~standard': {
    readonly version: 1
    readonly vendor: string
    readonly validate: (value: unknown) => StandardSchemaV1Result<Output> | Promise<StandardSchemaV1Result<Output>>
    readonly types?: { readonly input: Input; readonly output: Output }
  }
}

export type StandardSchemaV1Result<Output> =
  | { readonly value: Output; readonly issues?: undefined }
  | { readonly issues: readonly StandardSchemaV1Issue[] }

export interface StandardSchemaV1Issue {
  readonly message: string
  readonly path?: readonly (string | number | symbol)[]
}

export type ValidationMode = 'onBlur' | 'onChange' | 'onSubmit' | 'onTouched'

export interface FieldError {
  message: string
  path: string
}

export interface FormRootProps extends PrimitiveProps {
  schema?: StandardSchemaV1
  validationMode?: ValidationMode
  disabled?: boolean
}

export interface FormRootContext {
  disabled: boolean
  validationMode: ValidationMode
  errors: Ref<Map<string, string[]>>
  touched: Ref<Set<string>>
  dirty: Ref<Set<string>>
  isSubmitting: Ref<boolean>
  isValid: ComputedRef<boolean>
  values: Ref<Record<string, unknown>>
  setFieldValue: (name: string, value: unknown) => void
  setFieldTouched: (name: string) => void
  setFieldError: (name: string, errors: string[]) => void
  clearFieldError: (name: string) => void
  validateField: (name: string) => Promise<void>
  validate: () => Promise<boolean>
  reset: () => void
  registerField: (name: string, defaultValue?: unknown) => void
  unregisterField: (name: string) => void
}

const _formRootContext = createContext<FormRootContext>('FormRoot')
export const injectFormRootContext = _formRootContext[0]
export const provideFormRootContext = _formRootContext[1]

export interface FormFieldProps extends PrimitiveProps {
  name: string
  defaultValue?: unknown
}

export interface FormFieldContext {
  name: string
  id: string
  controlId: string
  messageId: string
  errors: ComputedRef<string[]>
  isTouched: ComputedRef<boolean>
  isDirty: ComputedRef<boolean>
  isInvalid: ComputedRef<boolean>
  value: ComputedRef<unknown>
  setValue: (value: unknown) => void
  setTouched: () => void
}

const _formFieldContext = createContext<FormFieldContext>('FormField')
export const injectFormFieldContext = _formFieldContext[0]
export const provideFormFieldContext = _formFieldContext[1]

export interface FormLabelProps extends PrimitiveProps {}
export interface FormControlProps extends PrimitiveProps {}
export interface FormMessageProps extends PrimitiveProps {
  match?: string | ((errors: string[]) => string | undefined)
}
export interface FormSubmitProps extends PrimitiveProps {
  disabled?: boolean
}
