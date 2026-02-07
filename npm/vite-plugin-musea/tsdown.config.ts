import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: ['src/index.ts', 'src/vrt.ts', 'src/cli.ts', 'src/a11y.ts', 'src/autogen.ts'],
  format: 'esm',
  dts: true,
  clean: false,
})
