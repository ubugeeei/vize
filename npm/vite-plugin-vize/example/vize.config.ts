import { defineConfig } from '@vizejs/vite-plugin'

// Static config example
export default defineConfig({
  compiler: {
    vapor: false,
    ssr: false,
    sourceMap: true,
    hoistStatic: true,
    cacheHandlers: true,
  },
  vite: {
    scanPatterns: ['src/**/*.vue'],
    ignorePatterns: ['node_modules/**', 'dist/**'],
  },
  linter: {
    enabled: true,
    categories: {
      correctness: 'error',
      suspicious: 'warn',
    },
  },
  typeChecker: {
    enabled: true,
    strict: false,
    checkProps: true,
    checkEmits: true,
  },
  formatter: {
    printWidth: 100,
    tabWidth: 2,
    semi: true,
    singleQuote: true,
    trailingComma: 'all',
  },
  musea: {
    include: ['src/**/*.art.vue'],
    basePath: '/__musea__',
    vrt: {
      threshold: 0.1,
      viewports: [{ width: 1280, height: 720, name: 'desktop' }],
    },
  },
  globalTypes: {
    $t: '(key: string) => string',
    $router: { type: 'import("vue-router").Router', defaultValue: 'undefined' },
  },
})

// Dynamic config example (uncomment to use):
//
// export default defineConfig((env) => ({
//   compiler: {
//     sourceMap: env.command === 'serve',
//     ssr: env.isSsrBuild,
//   },
//   linter: {
//     enabled: env.command !== 'build',
//   },
// }))
