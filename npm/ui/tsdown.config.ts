import { defineConfig } from 'tsdown'
import { vize } from '@vizejs/vite-plugin'

export default defineConfig({
  entry: ['src/index.ts', 'src/css/index.css'],
  format: 'esm',
  dts: { vue: true },
  clean: true,
  plugins: [vize()],
})
