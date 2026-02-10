import { defineConfig } from 'vite'
import { vize } from '@vizejs/vite-plugin'
import { musea } from '@vizejs/vite-plugin-musea'

export default defineConfig({
  plugins: [
    vize(),
    musea({
      include: ['src/**/*.vue', 'src/**/*.art.vue'],
      basePath: '/__musea__',
      inlineArt: true,
      css: ['./src/css/index.css'],
    }),
  ],
})
