import type { Component, VNode } from 'vue'

export type Direction = 'ltr' | 'rtl'
export type Orientation = 'horizontal' | 'vertical'
export type DataState = 'open' | 'closed' | 'checked' | 'unchecked' | 'indeterminate' | 'active' | 'inactive' | 'on' | 'off'

export type AcceptableValue = string | number | boolean | Record<string, unknown>

export interface PrimitiveProps {
  as?: string | Component
  asChild?: boolean
}

export interface FormFieldProps {
  id?: string
  name?: string
  disabled?: boolean
  required?: boolean
}

export type SingleOrMultiple<T = AcceptableValue> =
  | { type?: 'single'; modelValue?: T; defaultValue?: T }
  | { type: 'multiple'; modelValue?: T[]; defaultValue?: T[] }

export type VNodeRef = VNode | VNode[] | undefined
