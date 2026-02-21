---
title: Musea
---

# Musea

> **⚠️ Work in Progress:** Vize is under active development and is not yet ready for production use. Musea's API and art file format may change without notice.

Musea (*/mjuːˈziːə/*) is Vize's built-in component gallery — a Storybook alternative for exploring, documenting, and developing Vue components. The name is the plural of "museum": a place to display and appreciate works of art.

Unlike Storybook, which requires a separate build pipeline and its own configuration ecosystem, Musea is integrated directly into the Vize toolchain. It uses `*.art.vue` files to define component stories with a declarative, Vue-native syntax.

## Overview

![Musea Component Gallery — Home](/musea-home.png)

The gallery provides a dashboard showing all registered components, their variant counts, categories, tags, and status badges. Components are organized into categories (e.g., "Components", "Preview") and can be searched, filtered, and browsed from the sidebar.

## Installation

```bash
npm install @vizejs/vite-plugin-musea
```

## Usage

### Via CLI

```bash
vize musea
```

### Via Vite Plugin

```javascript
// vite.config.js
import { defineConfig } from 'vite';
import musea from '@vizejs/vite-plugin-musea';

export default defineConfig({
  plugins: [musea()],
});
```

### Via Nuxt

```typescript
// nuxt.config.ts
export default defineNuxtConfig({
  modules: ['@vizejs/nuxt'],
  vize: {
    musea: {
      include: ['**/*.art.vue'],
      tokensPath: 'assets/tokens.json',
      previewCss: ['assets/styles/main.css'],
      previewSetup: 'musea.preview.ts',
    },
  },
});
```

## Art Files

Musea uses `*.art.vue` files to define component stories. Each art file describes how a component should be displayed in the gallery, including its variants, metadata, and usage examples.

### Basic Structure

```vue
<script setup lang="ts">
import MyButton from './MyButton.vue'
</script>

<art
  title="MyButton"
  component="./MyButton.vue"
  category="Components"
  status="ready"
  tags="button, ui, input"
>
  <variant name="Default" default>
    <MyButton type="button">Click me</MyButton>
  </variant>

  <variant name="Outlined">
    <MyButton type="button" outlined>Click me</MyButton>
  </variant>

  <variant name="Icon">
    <MyButton type="button" icon>
      <svg><!-- icon --></svg>
    </MyButton>
  </variant>
</art>
```

### Art File Anatomy

| Element | Description |
|---------|-------------|
| `<script setup>` | Import the target component and any dependencies |
| `<art>` | Root element containing metadata and variants |
| `title` | Display name in the gallery |
| `component` | Relative path to the source component |
| `category` | Grouping in the sidebar (e.g., "Components", "Preview") |
| `status` | Component status badge: `ready`, `wip`, `deprecated` |
| `tags` | Comma-separated tags for search and filtering |
| `<variant>` | A named configuration of the component |
| `default` attribute | Marks the variant shown by default |

### Real-World Example

Here is an art file from the [Vue Fes Japan 2026](https://vuefes.jp/2026) conference website:

```vue
<script setup lang="ts">
import VFHeading from './VFHeading.vue'
</script>

<art
  title="VFHeading"
  component="./VFHeading.vue"
  category="Components"
  status="ready"
  tags="heading, typography"
>
  <variant name="Default" default>
    <VFHeading :level="2">Section Title</VFHeading>
  </variant>

  <variant name="H1">
    <VFHeading :level="1">Page Title</VFHeading>
  </variant>

  <variant name="H3">
    <VFHeading :level="3">Subsection Title</VFHeading>
  </variant>

  <variant name="With Anchor">
    <VFHeading :level="2" anchor="section-id">
      Linked Section
    </VFHeading>
  </variant>
</art>
```

## Component Detail View

![Musea Component Detail — Variants](/musea-component.png)

When you select a component from the sidebar, Musea shows a detail view with:

- **Component header** — Name, status badge, variant count, category, and tags
- **Preview toolbar** — Background color controls (Light, Dark, Transparent, custom), outline mode, and measurement overlay
- **Tab navigation** — Variants, Props, Docs, A11y, and VRT (Visual Regression Testing)
- **Variant selector** — Switch between variants defined in the art file
- **Live preview** — The component rendered in an isolated iframe
- **Source actions** — Copy template, view source, fullscreen, open in new tab

### Props Panel

![Musea Props Panel](/musea-props.png)

The **Props** tab provides an interactive props playground:

- **Live Preview** — Component updates in real-time as you change props
- **Props Controls** — Auto-generated controls based on the component's prop definitions (dropdowns for enums, toggles for booleans, text inputs for strings)
- **Slot editor** — Edit slot content directly with a code editor
- **Usage snippet** — Auto-generated template code reflecting the current prop values
- **Current Values** — JSON representation of all current prop values

## Design Tokens

![Musea Design Tokens](/musea-tokens.png)

Musea includes a **Design Tokens** viewer that reads from a JSON token file (e.g., Style Dictionary output). It displays:

- **Color tokens** — Visual swatches organized by category (Primary, Purple, Orange, Navy, etc.)
- **Typography tokens** — Font sizes and line heights with previews for each heading level and body text
- **Spacing and layout tokens** — Border radius, z-index, breakpoints
- **Semantic tokens** — Tokens that reference other tokens, showing the resolved values
- **Primitive vs. Semantic** classification — Filter tokens by type
- **Edit and delete** — Modify tokens directly from the gallery UI

Configure the tokens path in your Musea configuration:

```javascript
musea({
  tokensPath: 'assets/tokens.json',
})
```

The token file should follow the [Style Dictionary](https://amzn.github.io/style-dictionary/) format or a compatible JSON structure.

## Preview Configuration

Musea renders each variant in an isolated iframe. You can customize the preview environment:

### Preview CSS

Inject stylesheets into the preview iframe to ensure components render with your project's styles:

```javascript
musea({
  previewCss: [
    'assets/styles/main.css',
    'assets/styles/musea-preview.css',
  ],
})
```

### Preview Setup

Provide a setup function that runs before each preview. This is useful for installing plugins (vue-i18n, vue-router), registering global components, or configuring mock data:

```typescript
// musea.preview.ts
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import type { MuseaPreviewSetup } from '@vizejs/vite-plugin-musea'

export default ((app) => {
  // Install i18n
  const i18n = createI18n({
    locale: 'en',
    messages: { en: { /* ... */ }, ja: { /* ... */ } },
  })
  app.use(i18n)

  // Install router (with stub routes for NuxtLink compatibility)
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: { template: '<div />' } },
    ],
  })
  app.use(router)
}) satisfies MuseaPreviewSetup
```

## Features

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `⌘K` | Open component search |
| `⌘B` | Toggle sidebar |
| `Alt+O` | Toggle outline mode |
| `Alt+M` | Toggle measurement overlay |

### Component Search

The search bar (`⌘K`) provides fuzzy search across all component names and tags, allowing you to quickly navigate to any component in the gallery.

### Accessibility Audit (A11y)

The **A11y** tab runs accessibility checks on the rendered component, reporting issues based on WAI-ARIA best practices.

### Visual Regression Testing (VRT)

The **VRT** tab provides visual regression testing capabilities, allowing you to capture and compare component snapshots across changes.

## VS Code Support

The [Vize Art](https://github.com/ubugeeei/vize/tree/main/npm/vscode-art) VS Code extension provides syntax highlighting for `*.art.vue` files, making it easier to author story files with proper syntax support.

## AI Integration

Musea integrates with AI assistants through the [MCP server](../integrations/mcp.md). The MCP server exposes component metadata, story information, and gallery navigation to AI tools, enabling:

- AI-assisted component discovery and usage
- Automated documentation generation
- Intelligent code generation with correct props and slots
