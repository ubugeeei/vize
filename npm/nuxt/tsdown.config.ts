import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: ['src/index.ts'],
  format: 'esm',
  dts: true,
  clean: true,
  external: [
    '@vizejs/vite-plugin',
    '@vizejs/vite-plugin-musea',
    '@vizejs/musea-nuxt',
    'vize',
  ],
})
