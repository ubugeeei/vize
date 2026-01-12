<p align="center">
  <img src="./assets/logo.svg" alt="Vize Logo" width="400" />
</p>

<h1 align="center">Vize</h1>

<p align="center">
  <strong>Unofficial High-Performance Vue.js Toolchain in Rust</strong>
</p>

<p align="center">
  <em>/viːz/ — Named after Vizier + Visor + Advisor: a wise tool that sees through your code.</em>
</p>

<p align="center">
  <a href="https://vizejs.dev/play/"><strong>Playground</strong></a>
</p>

<p align="center">
  <a href="https://crates.io/crates/vize"><img src="https://img.shields.io/crates/v/vize.svg" alt="crates.io" /></a>
  <a href="https://www.npmjs.com/package/@vizejs/vite-plugin"><img src="https://img.shields.io/npm/v/@vizejs/vite-plugin.svg" alt="npm" /></a>
  <a href="https://www.npmjs.com/package/@vizejs/wasm"><img src="https://img.shields.io/npm/v/@vizejs/wasm.svg?label=@vizejs/wasm" alt="npm" /></a>
  <a href="https://github.com/ubugeeei/vize/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License" /></a>
</p>

> [!WARNING]
> This project is under active development and is not yet ready for production use.
> APIs and features may change without notice.

---

## Crates

<table>
  <tr>
    <th>Crate</th>
    <th>Description</th>
  </tr>
  <tr>
    <td><img src="./crates/vize_carton/logo.png" width="32" align="center" /> <a href="./crates/vize_carton">vize_carton</a></td>
    <td>Shared utilities & arena allocator</td>
  </tr>
  <tr>
    <td><img src="./crates/vize_relief/logo.png" width="32" align="center" /> <a href="./crates/vize_relief">vize_relief</a></td>
    <td>AST definitions, errors, options</td>
  </tr>
  <tr>
    <td><img src="./crates/vize_armature/logo.png" width="32" align="center" /> <a href="./crates/vize_armature">vize_armature</a></td>
    <td>Parser & tokenizer</td>
  </tr>
  <tr>
    <td><img src="./crates/vize_atelier_core/logo.png" width="32" align="center" /> <a href="./crates/vize_atelier_core">vize_atelier_core</a></td>
    <td>Transforms & code generation</td>
  </tr>
  <tr>
    <td><img src="./crates/vize_atelier_dom/logo.png" width="32" align="center" /> <a href="./crates/vize_atelier_dom">vize_atelier_dom</a></td>
    <td>DOM (VDom) compiler</td>
  </tr>
  <tr>
    <td><img src="./crates/vize_atelier_vapor/logo.png" width="32" align="center" /> <a href="./crates/vize_atelier_vapor">vize_atelier_vapor</a></td>
    <td>Vapor mode compiler</td>
  </tr>
  <tr>
    <td><img src="./crates/vize_atelier_sfc/logo.png" width="32" align="center" /> <a href="./crates/vize_atelier_sfc">vize_atelier_sfc</a></td>
    <td>SFC (.vue) compiler</td>
  </tr>
  <tr>
    <td><img src="./crates/vize_vitrine/logo.png" width="32" align="center" /> <a href="./crates/vize_vitrine">vize_vitrine</a></td>
    <td>Node.js / WASM bindings</td>
  </tr>
  <tr>
    <td><img src="./crates/vize/logo.png" width="32" align="center" /> <a href="./crates/vize">vize</a></td>
    <td>Command-line interface</td>
  </tr>
  <tr>
    <td><img src="./crates/vize_canon/logo.png" width="32" align="center" /> <a href="./crates/vize_canon">vize_canon</a></td>
    <td>TypeScript type checker</td>
  </tr>
  <tr>
    <td><img src="./crates/vize_patina/logo.png" width="32" align="center" /> <a href="./crates/vize_patina">vize_patina</a></td>
    <td>Vue.js linter</td>
  </tr>
  <tr>
    <td><img src="./crates/vize_glyph/logo.png" width="32" align="center" /> <a href="./crates/vize_glyph">vize_glyph</a></td>
    <td>Vue.js formatter</td>
  </tr>
  <tr>
    <td><img src="./crates/vize_maestro/logo.png" width="32" align="center" /> <a href="./crates/vize_maestro">vize_maestro</a></td>
    <td>Language Server Protocol</td>
  </tr>
  <tr>
    <td><img src="./crates/vize_musea/logo.png" width="32" align="center" /> <a href="./crates/vize_musea">vize_musea</a></td>
    <td>Component gallery (Storybook)</td>
  </tr>
</table>

## Naming Theme

Vize crates are named after **art and sculpture terminology**, reflecting how each component shapes and transforms Vue code:

| Name | Origin | Meaning |
|------|--------|---------|
| **Carton** | /kɑːˈtɒn/ | Artist's portfolio case — stores and organizes tools |
| **Relief** | /rɪˈliːf/ | Sculptural technique projecting from a surface — AST structure |
| **Armature** | /ˈɑːrmətʃər/ | Internal skeleton supporting a sculpture — parsing framework |
| **Atelier** | /ˌætəlˈjeɪ/ | Artist's workshop — compiler workspaces |
| **Vitrine** | /vɪˈtriːn/ | Glass display case — bindings exposing the compiler |
| **Canon** | /ˈkænən/ | Standard of ideal proportions — type checking |
| **Patina** | /ˈpætɪnə/ | Aged surface indicating quality — linting |
| **Glyph** | /ɡlɪf/ | Carved symbol or letterform — formatting |
| **Maestro** | /ˈmaɪstroʊ/ | Master conductor — LSP orchestration |
| **Musea** | /mjuːˈziːə/ | Plural of museum — component gallery |

## Architecture

<p align="center">
  <img src="./assets/architecture.png" alt="Vize Architecture" width="800" />
</p>

## Installation

### CLI

```bash
# via npm (recommended)
npm install -g vize

# via GitHub
npm install -g github:vizejs/vize

# via Cargo
cargo install vize
```

### npm

```bash
# WASM (Browser)
npm install @vizejs/wasm

# Vite Plugin
npm install @vizejs/vite-plugin
```

## Quick Start (Development)

### With mise (Recommended)

```bash
mise install && mise setup
mise cli      # Enable `vize` CLI command
mise dev      # Playground
```

### Without mise

<details>
<summary>Manual Setup</summary>

**Prerequisites:**

- [Rust](https://rustup.rs/) (1.80+)
- [Node.js](https://nodejs.org/) (20+)
- [pnpm](https://pnpm.io/) (9+)

**Setup:**

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install pnpm (if not already installed)
npm install -g pnpm

# Clone and setup
git clone https://github.com/ubugeeei/vize.git
cd vize
pnpm install

# Build CLI
cargo build --release -p vize
# The binary will be at: ./target/release/vize

# Or install to your PATH
cargo install --path crates/vize

# Run playground (optional)
pnpm -C playground dev
```

**Building Native Bindings (for @vizejs/native):**

```bash
cd npm/vize-native
pnpm build
```

**Building WASM (for @vizejs/wasm):**

```bash
# Install wasm-bindgen-cli
cargo install wasm-bindgen-cli

# Build WASM
cargo build --release -p vize_vitrine --no-default-features --features wasm --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/vize_bindings.wasm --out-dir npm/vize-wasm --target web
```

</details>

## Usage

### CLI

```bash
vize [COMMAND] [OPTIONS]
```

| Command | Description |
|---------|-------------|
| `build` | Compile Vue SFC files (default) |
| `fmt` | Format Vue SFC files |
| `lint` | Lint Vue SFC files |
| `check` | Type check Vue SFC files |
| `musea` | Start component gallery server |
| `lsp` | Start Language Server Protocol server |

```bash
vize --help           # Show help
vize <command> --help # Show command-specific help
```

**Examples:**

```bash
vize                              # Compile ./**/*.vue to ./dist
vize build src/**/*.vue -o out    # Custom input/output
vize build --ssr                  # SSR mode
vize build --script_ext=preserve  # Keep .ts/.tsx/.jsx extensions
vize fmt --check                  # Check formatting
vize lint --fix                   # Auto-fix lint issues
vize check --strict               # Strict type checking
```

### WASM (Browser)

```javascript
import init, { compileSfc } from '@vizejs/wasm';

await init();
const { code } = compileSfc(
  `<template><div>{{ msg }}</div></template>`,
  { filename: 'App.vue' }
);
```

### Vite Plugin

```javascript
// vite.config.js
import { defineConfig } from 'vite';
import vize from '@vizejs/vite-plugin';

export default defineConfig({
  plugins: [vize()],
});
```

## Performance

Compiling **15,000 SFC files** (36.9 MB):

|  | @vue/compiler-sfc | Vize | Speedup |
|--|-------------------|------|---------|
| **Single Thread** | 16.21s | 6.65s | **2.4x** |
| **Multi Thread** | 4.13s | 498ms | **8.3x** |

## Credits

This project is inspired by and builds upon the work of these amazing projects:

- [Volar.js](https://github.com/volarjs/volar.js) - The embedded language tooling framework
- [vuejs/language-tools](https://github.com/vuejs/language-tools) - Official Vue.js language tools
- [eslint-plugin-vue](https://github.com/vuejs/eslint-plugin-vue) - Official ESLint plugin for Vue.js
- [Storybook](https://github.com/storybookjs/storybook) - UI component explorer
- [OXC](https://github.com/oxc-project/oxc) - The JavaScript Oxidation Compiler

## License

MIT
