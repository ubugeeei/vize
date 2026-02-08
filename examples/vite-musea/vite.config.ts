import { defineConfig } from 'vite'
import vize from '@vizejs/vite-plugin'
import { musea } from '@vizejs/vite-plugin-musea'

export default defineConfig({
  base: process.env.CI ? '/musea-examples/' : '/',
  plugins: [
    vize(),
    musea({
      include: ['src/**/*.vue'],
      basePath: process.env.CI ? '/musea-examples/__musea__' : '/__musea__',
      inlineArt: true,
      tokensPath: 'src/tokens.json',
    }),
  ],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
})
