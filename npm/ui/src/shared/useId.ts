import { useId as vueUseId } from 'vue'

let count = 0

export function useId(deterministicId?: string): string {
  if (deterministicId) return deterministicId

  // Vue 3.5+ useId for SSR-safe IDs
  try {
    const id = vueUseId()
    if (id) return `vize-${id}`
  } catch {
    // fallback for Vue < 3.5
  }

  return `vize-${++count}`
}
