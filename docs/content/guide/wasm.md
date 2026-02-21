---
title: WASM Bindings
---

# WASM Bindings

> **⚠️ Work in Progress:** Vize is under active development and is not yet ready for production use. WASM APIs may change without notice.

`@vizejs/wasm` provides WebAssembly bindings for running the Vue compiler directly in the browser. This enables real-time SFC compilation, linting, and formatting without a server — ideal for playgrounds, documentation, and educational tools.

The WASM bindings are compiled from the same Rust codebase as the CLI and NAPI bindings (`vize_vitrine`), ensuring identical compilation output across all platforms.

## Installation

```bash
npm install @vizejs/wasm
```

## API

### Compile SFC

Compile a Vue Single File Component into JavaScript:

```javascript
import init, { compileSfc } from '@vizejs/wasm';

await init();

const result = compileSfc(
  `<template>
    <div>{{ msg }}</div>
  </template>

  <script setup lang="ts">
  const msg = ref('Hello Vize!')
  </script>`,
  { filename: 'App.vue' }
);

console.log(result.code);
// → import { ref, toDisplayString, ... } from 'vue'
// → ...compiled render function...
```

### Lint SFC

Run Vue-specific lint rules on an SFC:

```javascript
import init, { lintSfc } from '@vizejs/wasm';

await init();

const diagnostics = lintSfc(source, {
  filename: 'App.vue',
  locale: 'en', // 'en' | 'ja' | 'zh'
});

for (const d of diagnostics) {
  console.log(`${d.severity}: ${d.message} (line ${d.line})`);
}
```

### Format SFC

Format a Vue SFC:

```javascript
import init, { formatSfc } from '@vizejs/wasm';

await init();

const formatted = formatSfc(source, {
  filename: 'App.vue',
});

console.log(formatted);
```

## Initialization

The `init()` function must be called once before using any other API. It loads and instantiates the WebAssembly module:

```javascript
import init from '@vizejs/wasm';

// Basic initialization
await init();

// With custom WASM URL (useful for CDN or bundler setups)
await init('https://cdn.example.com/vize_vitrine_bg.wasm');
```

## Use Cases

### Playgrounds

Build interactive Vue compilation playgrounds that run entirely in the browser. The official [Vize Playground](https://vizejs.dev/play) uses the WASM bindings for real-time compilation:

```javascript
// React to editor changes and compile in real-time
editor.onChange((source) => {
  const { code, errors } = compileSfc(source, {
    filename: 'Playground.vue',
  });

  if (errors.length === 0) {
    preview.update(code);
  } else {
    diagnostics.show(errors);
  }
});
```

### Documentation

Embed live, editable Vue examples in your documentation:

```javascript
// Compile documentation examples on the fly
const examples = document.querySelectorAll('[data-vue-example]');
for (const el of examples) {
  const { code } = compileSfc(el.textContent, {
    filename: `example-${el.id}.vue`,
  });
  // Mount the compiled component...
}
```

### Education

Create interactive compiler exploration tools that show the compilation output in real-time, helping developers understand how Vue templates are transformed.

### CI/CD

Use WASM bindings for lightweight compilation in environments where native binaries are not available (e.g., Cloudflare Workers, Deno Deploy, browser-based CI).

## Building from Source

```bash
# Install wasm-bindgen-cli
cargo install wasm-bindgen-cli

# Build WASM
cargo build --release -p vize_vitrine \
  --no-default-features \
  --features wasm \
  --target wasm32-unknown-unknown

# Generate JS bindings
wasm-bindgen \
  target/wasm32-unknown-unknown/release/vize_vitrine.wasm \
  --out-dir npm/vize-wasm \
  --target web
```

## Internationalization

All WASM APIs that produce diagnostics (lint, compile errors) support localized messages:

| Code | Language |
|------|----------|
| `en` | English (default) |
| `ja` | Japanese (日本語) |
| `zh` | Chinese (中文) |

Pass the `locale` option to any API that produces diagnostics:

```javascript
const diagnostics = lintSfc(source, {
  filename: 'App.vue',
  locale: 'ja', // Lint messages in Japanese
});
```

## Bundle Size

The WASM module includes the full Vue compiler pipeline (parser, semantic analyzer, code generator) compiled to WebAssembly. The gzipped bundle size is approximately **1.5 MB**, which is suitable for non-critical-path loading (e.g., loaded after the page is interactive).

For production use, consider lazy-loading the WASM module:

```javascript
// Lazy-load the compiler only when needed
const compiler = await import('@vizejs/wasm');
await compiler.default(); // init()
const { code } = compiler.compileSfc(source, opts);
```
