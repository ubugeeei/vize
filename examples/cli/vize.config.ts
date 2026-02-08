import { defineConfig } from '@vizejs/vite-plugin'

export default defineConfig({
  compiler: {
    sourceMap: true,
  },
  linter: {
    enabled: true,
    categories: {
      correctness: 'error',
      suspicious: 'warn',
    },
  },
  formatter: {
    printWidth: 80,
    tabWidth: 2,
    semi: true,
    singleQuote: true,
    trailingComma: 'all',
  },
})
