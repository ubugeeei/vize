import type { ComputedRef, Ref } from 'vue'
import type { PrimitiveProps } from '../Primitive'
import type { Orientation } from '../shared'
import { createContext } from '../shared'

export interface StepperRootProps extends PrimitiveProps {
  modelValue?: number
  defaultValue?: number
  orientation?: Orientation
  linear?: boolean
}

export interface StepperRootContext {
  currentStep: ComputedRef<number>
  totalSteps: Ref<number>
  orientation: Orientation
  linear: boolean
  goToStep: (step: number) => void
  nextStep: () => void
  prevStep: () => void
  isFirstStep: ComputedRef<boolean>
  isLastStep: ComputedRef<boolean>
  registerStep: () => number
}

const _stepperRootContext = createContext<StepperRootContext>('StepperRoot')
export const injectStepperRootContext = _stepperRootContext[0]
export const provideStepperRootContext = _stepperRootContext[1]

export type StepState = 'active' | 'completed' | 'upcoming'

export interface StepperItemProps extends PrimitiveProps {
  step: number
  disabled?: boolean
  completed?: boolean
}

export interface StepperItemContext {
  step: number
  state: ComputedRef<StepState>
  disabled: boolean
  triggerId: string
}

const _stepperItemContext = createContext<StepperItemContext>('StepperItem')
export const injectStepperItemContext = _stepperItemContext[0]
export const provideStepperItemContext = _stepperItemContext[1]

export interface StepperTriggerProps extends PrimitiveProps {}
export interface StepperIndicatorProps extends PrimitiveProps {}
export interface StepperContentProps extends PrimitiveProps {
  forceMount?: boolean
}
export interface StepperSeparatorProps extends PrimitiveProps {}
