import { reactive, toRefs } from 'vue'

export interface ViewportPreset {
  name: string
  width: string
  height: string
}

export interface BackgroundPreset {
  name: string
  color: string
  pattern?: 'checkerboard'
}

export type GridDensity = 'compact' | 'comfortable' | 'spacious'

export const VIEWPORT_PRESETS: ViewportPreset[] = [
  { name: 'Reset', width: '100%', height: '100%' },
  { name: 'iPhone SE', width: '375px', height: '667px' },
  { name: 'iPhone 14', width: '390px', height: '844px' },
  { name: 'iPad', width: '768px', height: '1024px' },
  { name: 'iPad Pro', width: '1024px', height: '1366px' },
  { name: 'Desktop', width: '1280px', height: '800px' },
  { name: 'Wide', width: '1920px', height: '1080px' },
]

export const MULTI_VIEWPORT_PRESETS: ViewportPreset[] = [
  { name: 'Mobile', width: '375px', height: '667px' },
  { name: 'Tablet', width: '768px', height: '1024px' },
  { name: 'Desktop', width: '1280px', height: '800px' },
]

export const BG_PRESETS: BackgroundPreset[] = [
  { name: 'Light', color: '#ffffff' },
  { name: 'Dark', color: '#1a1a2e' },
  { name: 'Transparent', color: 'transparent', pattern: 'checkerboard' },
]

const state = reactive({
  background: null as BackgroundPreset | null,
  customBgColor: '',
  outlineEnabled: false,
  measureEnabled: false,
  viewport: VIEWPORT_PRESETS[0] as ViewportPreset,
  viewportRotated: false,
  gridDensity: 'comfortable' as GridDensity,
  multiViewportEnabled: false,
  fullscreenVariant: null as { artPath: string; variantName: string } | null,
})

export function useAddons() {
  function setBackground(preset: BackgroundPreset | null) {
    state.background = preset
    state.customBgColor = ''
  }

  function setCustomBackground(color: string) {
    state.background = null
    state.customBgColor = color
  }

  function getEffectiveBackground(): { color: string; pattern?: 'checkerboard' } {
    if (state.background) {
      return { color: state.background.color, pattern: state.background.pattern }
    }
    if (state.customBgColor) {
      return { color: state.customBgColor }
    }
    return { color: '' }
  }

  function toggleOutline() {
    state.outlineEnabled = !state.outlineEnabled
  }

  function toggleMeasure() {
    state.measureEnabled = !state.measureEnabled
  }

  function setViewport(preset: ViewportPreset) {
    state.viewport = preset
    state.viewportRotated = false
  }

  function rotateViewport() {
    state.viewportRotated = !state.viewportRotated
  }

  function getEffectiveViewport(): { width: string; height: string } {
    const vp = state.viewport
    if (state.viewportRotated && vp.width !== '100%') {
      return { width: vp.height, height: vp.width }
    }
    return { width: vp.width, height: vp.height }
  }

  function setGridDensity(density: GridDensity) {
    state.gridDensity = density
  }

  function toggleMultiViewport() {
    state.multiViewportEnabled = !state.multiViewportEnabled
  }

  function openFullscreen(artPath: string, variantName: string) {
    state.fullscreenVariant = { artPath, variantName }
  }

  function closeFullscreen() {
    state.fullscreenVariant = null
  }

  return {
    ...toRefs(state),
    setBackground,
    setCustomBackground,
    getEffectiveBackground,
    toggleOutline,
    toggleMeasure,
    setViewport,
    rotateViewport,
    getEffectiveViewport,
    setGridDensity,
    toggleMultiViewport,
    openFullscreen,
    closeFullscreen,
  }
}
