import type { ComputedRef, Ref } from 'vue'
import type { PrimitiveProps } from '../Primitive'
import type { Direction, Orientation } from '../shared'
import { createContext } from '../shared'

export interface AccordionRootProps extends PrimitiveProps {
  type?: 'single' | 'multiple'
  modelValue?: string | string[]
  defaultValue?: string | string[]
  disabled?: boolean
  collapsible?: boolean
  orientation?: Orientation
  dir?: Direction
}

export interface AccordionRootContext {
  type: 'single' | 'multiple'
  modelValue: ComputedRef<string | string[]>
  disabled: boolean
  collapsible: boolean
  orientation: Orientation
  dir: ComputedRef<Direction>
  changeValue: (itemValue: string) => void
  parentRef: Ref<HTMLElement | undefined>
}

const _accordionRootContext = createContext<AccordionRootContext>('AccordionRoot')
export const injectAccordionRootContext = _accordionRootContext[0]
export const provideAccordionRootContext = _accordionRootContext[1]

export interface AccordionItemProps extends PrimitiveProps {
  value: string
  disabled?: boolean
}

export interface AccordionItemContext {
  value: string
  triggerId: string
  contentId: string
  open: ComputedRef<boolean>
  disabled: ComputedRef<boolean>
}

const _accordionItemContext = createContext<AccordionItemContext>('AccordionItem')
export const injectAccordionItemContext = _accordionItemContext[0]
export const provideAccordionItemContext = _accordionItemContext[1]

export interface AccordionHeaderProps extends PrimitiveProps {}

export interface AccordionTriggerProps extends PrimitiveProps {}

export interface AccordionContentProps extends PrimitiveProps {
  forceMount?: boolean
}
