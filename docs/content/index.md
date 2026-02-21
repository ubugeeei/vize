---
layout: entry
title: Vize
description: Unofficial High-Performance Vue.js Toolchain in Rust. Compile, lint, format, type-check, and explore Vue components.
hero:
  name: Vize
  text: Unofficial High-Performance Vue.js Toolchain in Rust
  tagline: "/viːz/ — A wise tool that sees through your code. Compile, lint, format, type-check, and explore Vue components — all powered by Rust. ⚠️ Not yet production-ready."
  image:
    src: logo.svg
    alt: Vize Logo
  actions:
    - theme: brand
      text: Get Started
      link: getting-started.md
    - theme: alt
      text: GitHub
      link: https://github.com/ubugeeei/vize
    - theme: alt
      text: Playground
      link: https://vizejs.dev/play
features:
  - title: Blazing Fast CLI
    details: Compile, format, lint, and type-check Vue SFC files from a single Rust-native binary. One tool replaces an entire toolchain.
    link: guide/cli.md
  - title: Vite Plugin
    details: Drop-in replacement for @vitejs/plugin-vue with native compilation speed. No code changes required.
    link: guide/vite-plugin.md
  - title: 8.3x Faster
    details: Multi-threaded compilation of 15,000 SFC files (36.9 MB) in under 500ms. Arena allocation, Rayon parallelism, zero GC.
    link: architecture/performance.md
  - title: Component Gallery
    details: "Musea — a built-in Storybook alternative with art files, design tokens, props playground, a11y audit, and VRT."
    link: guide/musea.md
  - title: WASM Bindings
    details: Run the Vue compiler directly in the browser with WebAssembly. Power playgrounds, docs, and education tools.
    link: guide/wasm.md
  - title: AI Integration
    details: MCP server enabling AI assistants to understand and work with your Vue components through Musea.
    link: integrations/mcp.md
  - title: Vapor Mode
    details: First-class support for Vue 3.6 Vapor mode — fine-grained reactive compilation without the virtual DOM.
    link: architecture/overview.md
  - title: Philosophy
    details: "Art-inspired architecture, oxidation ecosystem (OXC, oxlint, tsgo), and a unified toolchain vision."
    link: philosophy.md
---

## Author

<img src="https://github.com/ubugeeei.png" alt="ubugeeei" width="80" style="border-radius: 50%;" />

**[ubugeeei](https://github.com/ubugeeei)** — Member of [Vue.js Japan User Group](https://github.com/vuejs-jp). Active in the Vue.js community as an organizer of [Vue Fes Japan](https://vuefes.jp/) and contributor to Vue.js ecosystem tools.

- GitHub: [github.com/ubugeeei](https://github.com/ubugeeei)
- X (Twitter): [@ubugeeei](https://x.com/ubugeeei)

## Sponsor

Vize is a free and open-source project licensed under MIT. Developing and maintaining a full toolchain — compiler, linter, formatter, type checker, LSP, component gallery, and WASM bindings — is a significant effort that requires sustained focus and dedication.

If Vize saves you time, improves your development experience, or you believe in the vision of a high-performance Vue.js toolchain, please consider sponsoring the project:

- [GitHub Sponsors](https://github.com/sponsors/ubugeeei)

Your support helps fund continued development, infrastructure costs, and ensures Vize remains free for everyone. Every contribution — no matter the size — makes a real difference.
