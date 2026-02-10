import { type InjectionKey, inject, provide } from 'vue'

export function createContext<T>(name: string, fallback?: T) {
  const key = Symbol(name) as InjectionKey<T>

  function provideContext(value: T) {
    provide(key, value)
    return value
  }

  function injectContext(consumerName?: string): T {
    const context = inject(key, fallback)
    if (context === undefined) {
      throw new Error(
        `\`${consumerName ?? name}\` must be used within \`${name}\``,
      )
    }
    return context
  }

  return [injectContext, provideContext] as const
}
