import { defineConfig } from 'vite'
import vize from '@vizejs/vite-plugin'
import path from 'node:path'

export default defineConfig({
  root: path.resolve(__dirname, 'gallery'),
  plugins: [vize()],
  build: {
    outDir: path.resolve(__dirname, 'dist/gallery'),
    emptyOutDir: true,
  },
})
