import { type Ref, computed, ref, watch } from 'vue'

export function useControllable<T>(
  props: Record<string, unknown>,
  key: string,
  emit: (event: string, ...args: unknown[]) => void,
  options: { defaultValue?: T } = {},
): Ref<T> {
  const internal = ref(options.defaultValue) as Ref<T>

  watch(
    () => props[key],
    (val) => {
      if (val !== undefined) {
        internal.value = val as T
      }
    },
    { immediate: true },
  )

  return computed({
    get() {
      return (props[key] !== undefined ? props[key] : internal.value) as T
    },
    set(val) {
      internal.value = val
      emit(`update:${key}`, val)
    },
  })
}
