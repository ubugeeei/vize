---
title: CLI
---

# CLI Reference

> **⚠️ Work in Progress:** Vize is under active development and is not yet ready for production use. Commands, options, and output formats may change without notice.

The Vize CLI provides a unified interface for all Vue.js toolchain operations. A single binary handles compilation, formatting, linting, type checking, component gallery, and language server — all powered by Rust.

## Installation

```bash
# via npm
npm install -g vize

# via Cargo
cargo install vize
```

## Usage

```bash
vize [COMMAND] [OPTIONS] [FILES...]
```

When invoked without a command, Vize defaults to `build`.

## Commands

| Command | Description | Crate |
|---------|-------------|-------|
| `build` | Compile Vue SFC files (default) | `vize_atelier_sfc` |
| `fmt` | Format Vue SFC files | `vize_glyph` |
| `lint` | Lint Vue SFC files | `vize_patina` |
| `check` | Type check Vue SFC files | `vize_canon` |
| `musea` | Start component gallery server | `vize_musea` |
| `lsp` | Start Language Server Protocol server | `vize_maestro` |

## Build

Compile Vue Single File Components into JavaScript. This is the default command — running `vize` without arguments is equivalent to `vize build`.

The build command supports three compilation backends:

- **DOM** (default) — Generates `createVNode` / `h` calls for Vue's virtual DOM runtime
- **Vapor** — Generates fine-grained reactive code without virtual DOM overhead (Vue 3.6+)
- **SSR** — Generates string concatenation code optimized for server-side rendering

```bash
# Default: compile ./**/*.vue to ./dist (DOM mode)
vize

# Custom input/output
vize build src/**/*.vue -o out

# SSR mode
vize build --ssr

# Preserve script extensions (.ts/.tsx/.jsx)
vize build --script_ext=preserve
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `-o, --output` | `dist` | Output directory |
| `--ssr` | `false` | Enable SSR mode (uses `vize_atelier_ssr`) |
| `--script_ext` | — | Script extension handling (`preserve` to keep `.ts`/`.tsx`) |

### Multi-threaded Compilation

The build command automatically uses all available CPU cores via Rayon. Each SFC file is compiled independently, achieving near-linear scaling:

| Files | Single Thread | Multi Thread (8 cores) | Speedup |
|-------|--------------|----------------------|---------|
| 1,000 | 443ms | 73ms | 6.1x |
| 5,000 | 2.2s | 198ms | 11.1x |
| 15,000 | 6.65s | 498ms | 13.4x |

## Format

Format Vue SFC files using the built-in formatter (Glyph). Glyph formats all three SFC blocks — `<template>`, `<script>`, and `<style>` — in a single pass.

```bash
# Format all Vue files in-place
vize fmt

# Check formatting without writing (exit code 1 if unformatted)
vize fmt --check

# Format specific files
vize fmt src/components/**/*.vue
```

### Options

| Option | Description |
|--------|-------------|
| `--check` | Check formatting without writing. Returns exit code 1 if any files are unformatted. |

## Lint

Lint Vue SFC files using the built-in linter (Patina). Patina provides Vue-specific lint rules covering template correctness, accessibility, and best practices.

```bash
# Lint all Vue files
vize lint

# Auto-fix lint issues
vize lint --fix

# Lint specific files
vize lint src/components/**/*.vue
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--fix` | `false` | Auto-fix lint issues where possible |
| `--locale` | `en` | Lint message language (`en`, `ja`, `zh`) |

### Locale Support

Lint messages are fully internationalized with support for three languages:

```bash
vize lint --locale ja   # Japanese (日本語)
vize lint --locale zh   # Chinese (中文)
vize lint --locale en   # English (default)
```

This is particularly useful for teams where English is not the primary language. Error messages, suggestions, and fix descriptions are all translated.

### Complementary with oxlint

Patina focuses on Vue-specific template rules. For broader JavaScript/TypeScript linting, Vize is designed to work alongside [oxlint](https://oxc.rs/docs/guide/usage/linter) — another Rust-native linter from the OXC project. The two tools are complementary:

- **Patina** — Vue template directives, accessibility, component best practices
- **oxlint** — JavaScript/TypeScript rules (no-unused-vars, no-console, etc.)

## Type Check

Type check Vue SFC files using the built-in checker (Canon). Canon performs TypeScript-aware type inference for template expressions, props, and emits.

```bash
# Type check
vize check

# Strict mode (stricter null checks and inference)
vize check --strict

# Check specific files
vize check src/components/**/*.vue
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--strict` | `false` | Enable strict type checking mode |

### Future: tsgo Integration

Vize's type checker is designed with awareness of [tsgo](https://github.com/nicolo-ribaudo/tsgo) (Microsoft's native TypeScript type checker written in Go). As tsgo matures, Vize plans to integrate with it for JavaScript/TypeScript type checking while continuing to provide Vue-specific template type analysis through Canon.

## Musea

Start the component gallery server for browsing and developing Vue components.

```bash
vize musea
```

This starts a development server with live reload, serving the Musea component gallery UI. See [Musea](./musea.md) for a comprehensive guide.

## LSP

Start the Language Server Protocol server for editor integration.

```bash
vize lsp
```

This starts the Maestro LSP server, which provides:

- IntelliSense and auto-completion
- Real-time diagnostics (compilation errors + lint warnings)
- Go to definition
- Hover information (types, props, documentation)
- Code actions and quick fixes

The LSP server is typically started automatically by the [VS Code extension](../integrations/vscode.md), but can also be used with any LSP-compatible editor (Neovim, Helix, Zed, etc.).

## Global Options

```bash
vize --help             # Show help
vize --version          # Show version
vize <command> --help   # Show command-specific help
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Error (compilation failure, lint errors, formatting issues with `--check`) |
