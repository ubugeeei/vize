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
  bgPrimary: '#0d0d0d',
  bgSecondary: '#1a1815',
  bgTertiary: '#252220',
  bgElevated: '#2d2a27',
  accent: '#a34828',
  accentHover: '#c45a32',
  accentSubtle: 'rgba(163, 72, 40, 0.15)',
  text: '#e6e9f0',
  textSecondary: '#c4c9d4',
  textMuted: '#7b8494',
  border: '#3a3530',
  borderSubtle: '#2a2725',
  success: '#4ade80',
  error: '#f87171',
  info: '#60a5fa',
  warning: '#fbbf24',
  shadow: '0 4px 24px rgba(0, 0, 0, 0.4)',
}

export const lightTheme: ThemeColors = {
  bgPrimary: '#ffffff',
  bgSecondary: '#f8f8f7',
  bgTertiary: '#f0eeed',
  bgElevated: '#ffffff',
  accent: '#a34828',
  accentHover: '#8b3d22',
  accentSubtle: 'rgba(163, 72, 40, 0.08)',
  text: '#1a1a1a',
  textSecondary: '#4a4a4a',
  textMuted: '#8b8b8b',
  border: '#e0dcd8',
  borderSubtle: '#ebe8e5',
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
  dark: darkTheme,
  light: lightTheme,
}
