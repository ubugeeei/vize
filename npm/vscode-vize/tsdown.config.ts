import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: ['src/extension.ts'],
  outDir: 'dist',
  format: 'cjs',
  platform: 'node',
  minify: true,
  external: ['vscode'],
})
