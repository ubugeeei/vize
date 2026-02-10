import { type Ref, computed, inject, provide, ref } from 'vue'
import type { Direction } from './types'

const DIRECTION_KEY = Symbol('Direction')

export function useDirection(dir?: Ref<Direction | undefined>) {
  const injectedDir = inject<Ref<Direction>>(DIRECTION_KEY, ref('ltr'))
  return computed(() => dir?.value ?? injectedDir.value)
}

export function provideDirection(dir: Ref<Direction>) {
  provide(DIRECTION_KEY, dir)
}
