/**
 * Built-in theme color palettes for Musea gallery.
 *
 * Each key maps to the CSS custom property name suffix:
 *   bgPrimary -> --musea-bg-primary
 */

export interface ThemeColors {
  bgPrimary: string
  bgSecondary: string
  bgTertiary: string
  bgElevated: string
  accent: string
  accentHover: string
  accentSubtle: string
  text: string
  textSecondary: string
  textMuted: string
  border: string
  borderSubtle: string
  success: string
  error: string
  info: string
  warning: string
  shadow: string
}

export const darkTheme: ThemeColors = {
  bgPrimary: '#121212',
  bgSecondary: '#1a1a1a',
  bgTertiary: '#242424',
  bgElevated: '#2a2a2a',
  accent: '#E6E2D6',
  accentHover: '#d4d0c4',
  accentSubtle: 'rgba(230, 226, 214, 0.1)',
  text: '#E6E2D6',
  textSecondary: '#c4c1b6',
  textMuted: '#8a8880',
  border: '#2e2e2e',
  borderSubtle: '#232323',
  success: '#4ade80',
  error: '#f87171',
  info: '#60a5fa',
  warning: '#fbbf24',
  shadow: '0 4px 24px rgba(0, 0, 0, 0.4)',
}

export const lightTheme: ThemeColors = {
  bgPrimary: '#E6E2D6',
  bgSecondary: '#ddd9cd',
  bgTertiary: '#d4d0c4',
  bgElevated: '#E6E2D6',
  accent: '#121212',
  accentHover: '#2a2a2a',
  accentSubtle: 'rgba(18, 18, 18, 0.08)',
  text: '#121212',
  textSecondary: '#3a3a3a',
  textMuted: '#6b6b6b',
  border: '#c8c4b8',
  borderSubtle: '#d4d0c4',
  success: '#16a34a',
  error: '#dc2626',
  info: '#2563eb',
  warning: '#d97706',
  shadow: '0 4px 24px rgba(0, 0, 0, 0.08)',
}

/** Map from camelCase color key to CSS custom property name. */
export function colorKeyToCssVar(key: string): string {
  // bgPrimary -> --musea-bg-primary
  const kebab = key.replace(/([A-Z])/g, '-$1').toLowerCase()
  return `--musea-${kebab}`
}

export const builtInThemes: Record<string, ThemeColors> = {
  light: lightTheme,
  dark: darkTheme,
}
