import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'node:path'

export default defineConfig({
  root: path.resolve(__dirname, 'gallery'),
  base: '/__musea__/',
  plugins: [vue()],
  build: {
    outDir: path.resolve(__dirname, 'dist/gallery'),
    emptyOutDir: true,
  },
})
