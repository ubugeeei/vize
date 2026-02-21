---
title: Getting Started
---

# Getting Started

> **⚠️ Work in Progress:** Vize is under active development and is not yet ready for production use. APIs and features may change without notice.

## What is Vize?

Vize (*/viːz/*) is an unofficial, high-performance Vue.js toolchain written entirely in Rust. It provides a unified set of tools for Vue.js development:

| Tool | Purpose | Replaces |
|------|---------|----------|
| `vize build` | SFC compilation | `@vue/compiler-sfc` |
| `vize lint` | Template and script linting | `eslint-plugin-vue` |
| `vize fmt` | Code formatting | `prettier` |
| `vize check` | Type checking | `vue-tsc` |
| `vize musea` | Component gallery | Storybook |
| `vize lsp` | Editor integration | Volar |

All of these share a single parser, a single AST representation, and a single configuration surface — eliminating the overhead and inconsistencies of maintaining separate tools.

## Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [pnpm](https://pnpm.io/) 9+ (recommended) or npm

For building from source:

- [Rust](https://rustup.rs/) 1.80+

## Installation

### CLI

Install the Vize CLI globally:

```bash
# via npm (recommended)
npm install -g vize

# via Cargo (requires --version for pre-release)
cargo install vize --version 0.0.1-alpha.120
```

### npm Packages

Vize is distributed as multiple npm packages, each serving a specific integration point:

```bash
# Main package (includes CLI)
npm install vize

# Native bindings (Node.js) — used by the Vite plugin
npm install @vizejs/native

# WASM bindings (Browser) — for playgrounds and in-browser compilation
npm install @vizejs/wasm

# Vite plugin — drop-in replacement for @vitejs/plugin-vue
npm install @vizejs/vite-plugin

# Nuxt module — first-class Nuxt integration
npm install @vizejs/nuxt

# Musea (component gallery)
npm install @vizejs/vite-plugin-musea

# MCP server (AI assistant integration)
npm install @vizejs/musea-mcp-server
```

## Quick Start

### Using the CLI

Once installed, you can compile Vue SFC files immediately:

```bash
# Compile all .vue files in current directory to ./dist
vize

# Custom input/output
vize build src/**/*.vue -o out

# SSR mode
vize build --ssr

# Format check
vize fmt --check

# Lint with auto-fix
vize lint --fix

# Type check
vize check --strict
```

### Using the Vite Plugin

Add Vize to your Vite project for native-speed Vue compilation. This is a drop-in replacement for `@vitejs/plugin-vue` — no code changes to your components are required:

```javascript
// vite.config.js
import { defineConfig } from 'vite';
import vize from '@vizejs/vite-plugin';

export default defineConfig({
  plugins: [vize()],
});
```

The plugin handles SFC compilation, `<script setup>`, scoped CSS, HMR, and SSR — all through Rust-native NAPI bindings. See [Vite Plugin](./guide/vite-plugin.md) for configuration options.

### Using with Nuxt

Vize provides a dedicated Nuxt module with first-class support:

```typescript
// nuxt.config.ts
export default defineNuxtConfig({
  modules: ['@vizejs/nuxt'],
  vize: {
    compiler: true,
    musea: {
      include: ['**/*.art.vue'],
    },
  },
});
```

See [Nuxt Integration](./integrations/nuxt.md) for more details.

### Using WASM in the Browser

```javascript
import init, { compileSfc } from '@vizejs/wasm';

await init();
const { code } = compileSfc(
  `<template><div>{{ msg }}</div></template>`,
  { filename: 'App.vue' }
);
```

See [WASM Bindings](./guide/wasm.md) for the full API.

## Development Setup

For contributing to Vize itself:

### With mise (Recommended)

```bash
mise install && mise setup
mise cli      # Enable vize CLI command
mise dev      # Start playground
```

### Manual Setup

```bash
git clone https://github.com/ubugeeei/vize.git
cd vize
pnpm install

# Build CLI
cargo build --release -p vize

# Run playground
pnpm -C playground dev
```

### Project Structure

```
vize/
├── crates/               # Rust crates (compiler, linter, formatter, etc.)
│   ├── vize/             # CLI binary
│   ├── vize_armature/    # Parser
│   ├── vize_relief/      # AST definitions
│   ├── vize_croquis/     # Semantic analysis
│   ├── vize_atelier_*/   # Compilation backends (dom, vapor, sfc, ssr)
│   ├── vize_patina/      # Linter
│   ├── vize_glyph/       # Formatter
│   ├── vize_canon/       # Type checker
│   ├── vize_maestro/     # LSP
│   ├── vize_musea/       # Component gallery
│   ├── vize_vitrine/     # NAPI + WASM bindings
│   └── ...
├── npm/                  # npm packages (vite-plugin, wasm, native, etc.)
├── playground/           # Development playground
├── docs/                 # Documentation (this site)
└── tests/                # Integration tests
```

## Next Steps

- [Philosophy](./philosophy.md) — Design principles and vision
- [CLI Reference](./guide/cli.md) — Full command documentation
- [Vite Plugin](./guide/vite-plugin.md) — Configuration options
- [Musea](./guide/musea.md) — Component gallery guide
- [Architecture](./architecture/overview.md) — How Vize works internally
