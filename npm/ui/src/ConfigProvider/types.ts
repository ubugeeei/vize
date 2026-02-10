import type { Direction } from '../shared'

export interface ConfigProviderProps {
  dir?: Direction
  useId?: (deterministicId?: string) => string
  scrollBody?: boolean
  nonce?: string
}

export interface ConfigProviderContext {
  dir: Direction
  useId?: (deterministicId?: string) => string
  scrollBody: boolean
  nonce?: string
}
