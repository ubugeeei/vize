import { ref } from 'vue'
import type { PaletteApiResponse } from '../api'
import { fetchPalette } from '../api'

export function usePalette() {
  const palette = ref<PaletteApiResponse | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const values = ref<Record<string, unknown>>({})

  async function load(artPath: string) {
    loading.value = true
    error.value = null
    try {
      palette.value = await fetchPalette(artPath)
      // Initialize values from defaults
      const initial: Record<string, unknown> = {}
      for (const control of palette.value.controls) {
        if (control.default_value !== undefined) {
          initial[control.name] = control.default_value
        }
      }
      values.value = initial
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      palette.value = null
    } finally {
      loading.value = false
    }
  }

  function setValue(name: string, value: unknown) {
    values.value = { ...values.value, [name]: value }
  }

  function resetValues() {
    if (!palette.value) return
    const initial: Record<string, unknown> = {}
    for (const control of palette.value.controls) {
      if (control.default_value !== undefined) {
        initial[control.name] = control.default_value
      }
    }
    values.value = initial
  }

  return {
    palette,
    loading,
    error,
    values,
    load,
    setValue,
    resetValues,
  }
}
