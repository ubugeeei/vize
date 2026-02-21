import { ref, computed, type Ref, type ComputedRef } from 'vue'
import { builtInThemes, colorKeyToCssVar, type ThemeColors } from '../themes'

declare global {
  interface Window {
    __MUSEA_THEME_CONFIG__?: ThemeConfig
  }
}

export interface ThemeConfig {
  /** Initial theme: 'dark' | 'light' | 'system' | custom name */
  default: string
  /** Custom themes keyed by name */
  custom?: Record<string, { base?: 'dark' | 'light'; colors: Partial<ThemeColors> }>
}

const STORAGE_KEY = 'musea-theme'

// Singleton state so all components share the same theme
let initialized = false
const currentTheme: Ref<string> = ref('light')

const resolvedTheme: ComputedRef<string> = computed(() => {
  if (currentTheme.value === 'system') {
    return typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: light)').matches
      ? 'light'
      : 'dark'  // system: fallback to dark when OS prefers dark
  }
  return currentTheme.value
})

/** All available theme names that the user can cycle through. */
const availableThemes: Ref<string[]> = ref(['light', 'dark', 'system'])

function getConfig(): ThemeConfig {
  return (typeof window !== 'undefined' && window.__MUSEA_THEME_CONFIG__) || { default: 'light' }
}

function applyTheme(name: string): void {
  const el = document.documentElement
  const config = getConfig()

  // Check if this is a custom theme
  const customDef = config.custom?.[name]
  if (customDef) {
    const base = customDef.base ?? 'dark'
    // Set data attribute to the base theme for CSS variable defaults
    el.setAttribute('data-musea-theme', base)
    // Apply custom color overrides as inline styles
    const baseColors = builtInThemes[base]
    const merged = { ...baseColors, ...customDef.colors }
    for (const [key, value] of Object.entries(merged)) {
      el.style.setProperty(colorKeyToCssVar(key), value)
    }
  } else {
    // Built-in or system theme
    el.setAttribute('data-musea-theme', name)
    // Clear any custom inline overrides
    clearCustomStyles()
  }
}

function clearCustomStyles(): void {
  const el = document.documentElement
  // Remove all --musea-* inline styles
  const style = el.style
  const toRemove: string[] = []
  for (let i = 0; i < style.length; i++) {
    const prop = style[i]
    if (prop.startsWith('--musea-')) {
      toRemove.push(prop)
    }
  }
  for (const prop of toRemove) {
    style.removeProperty(prop)
  }
}

let mediaQuery: MediaQueryList | null = null
let mediaHandler: ((e: MediaQueryListEvent) => void) | null = null

export function useTheme() {
  if (!initialized && typeof window !== 'undefined') {
    initialized = true
    const config = getConfig()

    // Build available themes list
    const themes = ['light', 'dark', 'system']
    if (config.custom) {
      for (const name of Object.keys(config.custom)) {
        if (!themes.includes(name)) {
          themes.push(name)
        }
      }
    }
    availableThemes.value = themes

    // Determine initial theme: localStorage > config default > 'light'
    const stored = localStorage.getItem(STORAGE_KEY)
    const initial = stored && themes.includes(stored) ? stored : config.default
    currentTheme.value = initial

    applyTheme(initial)

    // Listen for OS color-scheme changes when in system mode
    mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    mediaHandler = () => {
      if (currentTheme.value === 'system') {
        applyTheme('system')
      }
    }
    mediaQuery.addEventListener('change', mediaHandler)
  }

  function setTheme(name: string): void {
    currentTheme.value = name
    applyTheme(name)
    localStorage.setItem(STORAGE_KEY, name)
  }

  function cycleTheme(): void {
    const themes = availableThemes.value
    const idx = themes.indexOf(currentTheme.value)
    const next = themes[(idx + 1) % themes.length]
    setTheme(next)
  }

  return {
    currentTheme: currentTheme as Readonly<Ref<string>>,
    resolvedTheme,
    availableThemes: availableThemes as Readonly<Ref<string[]>>,
    setTheme,
    cycleTheme,
  }
}
