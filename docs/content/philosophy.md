---
title: Philosophy
---

# Philosophy

> **⚠️ Work in Progress:** Vize is under active development and is not yet ready for production use. The design principles below describe the project's vision and direction.

Vize is more than a compiler — it is a design statement about how Vue.js tooling should work.

## Why Vize Exists

The JavaScript ecosystem has long relied on JavaScript-based tooling to compile, lint, format, and type-check JavaScript code. This creates a fundamental bottleneck: the tools that process your code are subject to the same runtime limitations as the code they process — garbage collection pauses, single-threaded execution, and dynamic dispatch overhead.

Vize takes a different approach. By rewriting the entire Vue.js toolchain in Rust, we eliminate these constraints at the architecture level. The result is not an incremental improvement — it is a categorical shift in what is possible.

## Design Principles

### 1. Unified Toolchain

Traditional Vue.js development requires assembling a constellation of separate tools: a compiler (`@vue/compiler-sfc`), a linter (eslint + eslint-plugin-vue), a formatter (prettier), a type checker (vue-tsc), and a component explorer (Storybook). Each tool has its own parser, its own AST representation, and its own configuration format.

Vize unifies all of these into a single binary. One parser. One AST. One configuration surface. This eliminates redundant parsing passes, reduces configuration complexity, and ensures that all tools share a consistent understanding of your code.

```
@vue/compiler-sfc  +  eslint-plugin-vue  +  prettier  +  vue-tsc  +  Storybook
                              ↓
                            vize
```

### 2. Performance as a Feature

Speed is not a nice-to-have — it is a prerequisite for developer experience. When compilation takes seconds, developers lose flow. When linting takes minutes, developers disable it. When type checking takes too long, developers skip it.

Vize is designed so that every tool runs fast enough to be used interactively:

- **Compilation**: 15,000 SFC files in 498ms (multi-threaded)
- **Formatting**: Near-instant, even on large codebases
- **Linting**: Real-time feedback through the LSP
- **Type checking**: Incremental analysis without V8 overhead

This is achieved through Rust's zero-cost abstractions, arena allocation, and native multi-threading with Rayon.

### 3. Drop-in Compatibility

Vize does not ask you to rewrite your code or change your workflow. The Vite plugin is a drop-in replacement for `@vitejs/plugin-vue`. Your existing Vue components, `<script setup>`, scoped styles, and HMR all work without modification.

This principle extends to the broader ecosystem. Vize's Vite plugin is compatible with Nuxt, and the LSP integrates with VS Code through standard protocols. Adopting Vize should feel like upgrading your engine, not rebuilding your car.

### 4. Art as Architecture

Every Vize crate is named after a concept from the visual arts — painting, sculpture, and museum curation. This is not mere whimsy. The naming convention encodes a philosophy: **code is a creative medium**, and the tools that shape it should reflect the craft involved.

| Crate | Art Origin | Role |
|-------|-----------|------|
| **Carton** | Artist's portfolio case | Shared utilities — the toolbox |
| **Relief** | Sculptural surface projection | AST — the structured surface of code |
| **Armature** | Skeleton supporting a sculpture | Parser — the structural framework |
| **Croquis** | Quick gestural sketch | Semantic analysis — capturing essence |
| **Atelier** | Artist's workshop | Compiler — where transformation happens |
| **Vitrine** | Glass display case | Bindings — exposing the work |
| **Canon** | Standard of ideal proportions | Type checker — ensuring correctness |
| **Patina** | Aged surface indicating quality | Linter — polishing the surface |
| **Glyph** | Carved symbol or letterform | Formatter — shaping the text |
| **Maestro** | Master conductor | LSP — orchestrating the experience |
| **Musea** | Plural of museum | Component gallery — exhibiting the work |
| **Fresco** | Wall painting technique | TUI framework — painting the terminal |

This naming system serves a practical purpose: it makes the crate hierarchy intuitive. When you see `vize_atelier_dom`, you immediately understand it is a *workshop* that produces *DOM output*. When you see `vize_patina`, you know it *polishes* your code.

#### The Sculpture Analogy

The deepest analogy is between software compilation and sculpture. Consider how a sculptor works:

1. **Armature** — The sculptor begins by constructing an armature: a wire skeleton that defines the basic structure. In Vize, the parser (`vize_armature`) constructs the structural framework (AST) from raw source text.

2. **Relief** — The sculptor builds the surface on top of the armature, creating a *relief* — a structured surface that projects from a flat plane. In Vize, the AST (`vize_relief`) gives structured, three-dimensional form to what was originally flat text.

3. **Croquis** — Before committing to a final sculpture, the artist makes quick sketches (*croquis*) to understand the subject's essential character. In Vize, semantic analysis (`vize_croquis`) is a quick pass that captures the meaning of code — what variables are bound, what expressions are valid — without committing to a compilation target.

4. **Atelier** — The sculptor moves to the *atelier* (workshop) to create the final piece. Multiple ateliers may produce different renditions of the same subject. In Vize, the compilation backends (`vize_atelier_dom`, `vize_atelier_vapor`, `vize_atelier_ssr`) are different workshops that produce different renditions (DOM, Vapor, SSR) of the same analyzed AST.

5. **Vitrine** — The finished work is placed in a *vitrine* (glass display case) so others can observe it. In Vize, the bindings (`vize_vitrine`) are a transparent layer that lets JavaScript consumers access the compiled output.

6. **Musea** — Finally, the works are exhibited in a *museum* for appreciation and study. In Vize, the component gallery (`vize_musea`) is where components are exhibited, explored, and documented.

#### The Quality Crafts Analogy

The remaining crates follow a craftsmanship analogy:

- **Canon** (type checker) — In classical sculpture, the *canon* was a standard of ideal human proportions. Polykleitos wrote the *Kanon* defining mathematical ratios for the perfect figure. In Vize, the type checker enforces the "ideal proportions" of your code — types must be correct, props must match, emissions must conform.

- **Patina** (linter) — A *patina* is the surface finish that develops on aged materials, indicating quality and care. A bronze sculpture with a rich patina has been well-maintained. In Vize, the linter examines the surface of your code, identifying issues that affect its quality.

- **Glyph** (formatter) — A *glyph* is a carved symbol or letterform — think of the precise, consistent letterforms in a font. Each glyph has exact proportions and spacing. In Vize, the formatter ensures your code has consistent, precise proportions.

- **Maestro** (LSP) — A *maestro* is the master conductor who orchestrates an ensemble into a unified performance. In Vize, the LSP server orchestrates all language features (completion, diagnostics, formatting, navigation) into a unified editor experience.

- **Fresco** (TUI) — A *fresco* is a painting technique where pigment is applied to wet plaster, becoming part of the wall itself. In Vize, the TUI framework "paints" interfaces directly onto the terminal surface.

### 5. Vapor-First Thinking

Vue 3.6 introduces Vapor mode — a compilation strategy that generates fine-grained reactive code without the virtual DOM. Vize was designed with Vapor mode as a first-class compilation target from day one.

While `@vue/compiler-sfc` added Vapor support incrementally, Vize's `vize_atelier_vapor` was built alongside `vize_atelier_dom` from the beginning. This means the shared compilation infrastructure (`vize_atelier_core`) is designed to serve both output modes equally well.

### 6. Developer Sovereignty

Vize is an **unofficial** toolchain. It is not controlled by the Vue.js core team, and it makes no claim to be the "official" way to build Vue applications. This is intentional.

By remaining independent, Vize can:

- Experiment with compilation strategies without the burden of backwards compatibility
- Move faster than an official project bound by governance processes
- Serve as a proving ground for ideas that may eventually influence the official toolchain
- Provide an alternative for developers who want maximum performance

At the same time, Vize tracks the official Vue.js specification closely. The goal is compatibility, not fragmentation.

### 7. Standing on the Shoulders of Oxidation

Vize does not exist in isolation. It is part of a broader movement to rewrite JavaScript tooling in systems languages — what the community calls "oxidation." Vize embraces and integrates with this ecosystem:

- **OXC** — Vize uses the [Oxidation Compiler](https://oxc.rs/) (oxc) for JavaScript and TypeScript parsing. OXC provides the high-performance JS/TS AST parsing that powers `vize_croquis` (semantic analysis) and `vize_atelier_core` (code generation). Rather than reimplement a JS parser, Vize delegates to OXC's battle-tested implementation.
- **oxlint** — Vize is designed with [oxlint](https://oxc.rs/docs/guide/usage/linter) in mind. While `vize_patina` handles Vue-specific template linting, the broader JavaScript linting story is best served by oxlint's Rust-native rule engine. The two tools are complementary, not competing.
- **tsgo (TypeScript Go)** — Microsoft's [native TypeScript type checker](https://github.com/nicolo-ribaudo/tsgo) (the Go port of TypeScript, previously known as `@typescript/native-preview`) represents a future where type checking is no longer bottlenecked by JavaScript. Vize's `vize_canon` is designed with awareness of this trajectory — as tsgo matures, Vize will integrate with it for JavaScript/TypeScript type checking while continuing to provide Vue-specific template type analysis.
- **LightningCSS** — Vize uses [LightningCSS](https://lightningcss.dev/) for CSS parsing and transformation within `vize_atelier_sfc`, leveraging its Rust-native CSS processing for scoped styles.

There are still many unsolved challenges in this space — cross-tool AST interop, incremental analysis across language boundaries, and editor integration consistency. Vize aims to be a proving ground for solutions to these problems within the Vue.js ecosystem, contributing to the broader oxidation movement.

## The Name

**Vize** (*/viːz/*) is derived from three words:

- **Vizier** — a wise counselor or advisor
- **Visor** — something that helps you see clearly
- **Advisor** — a guide that helps you make better decisions

Together, they describe a tool that *sees through your code* and *advises you wisely*. The pronunciation rhymes with "breeze" — fast, effortless, and refreshing.
