---
title: Nuxt
---

# Nuxt Integration

> **⚠️ Work in Progress:** Vize is under active development and is not yet ready for production use. Test thoroughly before adopting in Nuxt projects.

Vize provides first-class Nuxt integration through the `@vizejs/nuxt` module. This replaces Nuxt's default Vue compiler with Vize's Rust-native compiler, providing the same speed improvements in Nuxt projects.

## Installation

```bash
npm install @vizejs/nuxt
```

## Setup

### Using the Nuxt Module (Recommended)

```typescript
// nuxt.config.ts
export default defineNuxtConfig({
  modules: ['@vizejs/nuxt'],
  vize: {
    compiler: true,
  },
});
```

### Using the Vite Plugin Directly

Alternatively, you can use the Vite plugin directly. Since Nuxt uses Vite under the hood, this works but lacks some Nuxt-specific optimizations:

```typescript
// nuxt.config.ts
import vize from '@vizejs/vite-plugin';

export default defineNuxtConfig({
  vite: {
    plugins: [vize()],
  },
});
```

## Musea Integration

The Nuxt module also supports Musea (component gallery) integration:

```typescript
// nuxt.config.ts
export default defineNuxtConfig({
  modules: ['@vizejs/nuxt'],
  vize: {
    compiler: true,
    musea: {
      include: ['**/*.art.vue'],
      tokensPath: 'assets/tokens.json',
      previewCss: [
        'assets/styles/main.css',
        'assets/styles/musea-preview.css',
      ],
      previewSetup: 'musea.preview.ts',
    },
    nuxtMusea: {
      route: { path: '/' }, // Musea UI route within __musea__
    },
  },
});
```

When configured, the Musea gallery is available at `/__musea__/` during development.

### Preview Setup for Nuxt

Nuxt projects often use features that need to be mocked in the Musea preview environment (vue-i18n, NuxtLink, useNuxtApp, etc.):

```typescript
// musea.preview.ts
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import type { MuseaPreviewSetup } from '@vizejs/vite-plugin-musea'

export default ((app) => {
  // Mock vue-i18n
  const i18n = createI18n({
    locale: 'ja',
    messages: { ja: { /* ... */ }, en: { /* ... */ } },
  })
  app.use(i18n)

  // Mock vue-router (for NuxtLink compatibility)
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: { template: '<div />' } },
      { path: '/about', component: { template: '<div />' } },
    ],
  })
  app.use(router)

  // Register NuxtLink as RouterLink
  app.component('NuxtLink', app.component('RouterLink'))

  // Mock useNuxtApp if needed
  app.provide('nuxt-app', {
    $config: { public: { /* ... */ } },
  })
}) satisfies MuseaPreviewSetup
```

## How It Works

When the Nuxt module is installed:

1. **Vite plugin injection** — The module registers `@vizejs/vite-plugin` as a Vite plugin, intercepting `.vue` file compilation.
2. **Compatibility shim** — The plugin exposes a `@vitejs/plugin-vue` compatibility API, so Nuxt's internal checks (which probe for the Vue plugin) work correctly.
3. **SSR support** — Vize's `vize_atelier_ssr` handles server-side compilation. The plugin isolates client and server environment variables to prevent cross-contamination.
4. **Nuxt features preserved** — Auto-imports, composables, middleware, and other Nuxt features work through Nuxt's own transform layer, which runs after Vize's compilation.

## Real-World Example

The [Vue Fes Japan 2026](https://vuefes.jp/2026) conference website uses Vize with Nuxt 4:

```typescript
// nuxt.config.ts
export default defineNuxtConfig({
  modules: ['@vizejs/nuxt'],
  vize: {
    compiler: false, // compiler disabled (using Nuxt's default)
    musea: {
      include: ['**/*.art.vue'],
      inlineArt: false,
      tokensPath: 'assets/tokens.json',
      previewCss: [
        'assets/styles/main.css',
        'assets/styles/musea-preview.css',
      ],
      previewSetup: 'musea.preview.ts',
    },
  },
});
```

This configuration uses Musea for component development and documentation while keeping Nuxt's default compiler for production builds.

## Notes

- Vize is under active development — test thoroughly before using in production Nuxt projects
- SSR compilation is supported via `vize_atelier_ssr`
- Nuxt-specific features (auto-imports, composables, middleware) work through Nuxt's own transform layer
- The Nuxt module supports both Nuxt 3 and Nuxt 4
