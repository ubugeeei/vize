import { defineConfig } from '@vizejs/vite-plugin'

export default defineConfig({
  compiler: {
    vapor: false,
    ssr: false,
    sourceMap: true,
  },
  vite: {
    scanPatterns: ['src/**/*.vue'],
    ignorePatterns: ['node_modules/**', 'dist/**'],
  },
  musea: {
    include: ['src/**/*.vue'],
    basePath: '/__musea__',
    inlineArt: true,
    vrt: {
      threshold: 0.1,
      viewports: [{ width: 1280, height: 720, name: 'desktop' }],
    },
  },
})
