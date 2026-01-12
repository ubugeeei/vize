<p align="center">
  <img src="https://raw.githubusercontent.com/ubugeeei/vize/main/crates/vize_croquis/logo.svg" alt="vize_croquis logo" width="120" height="120">
</p>

<h1 align="center">vize_croquis</h1>

<p align="center">
  <strong>Croquis - Quick sketches of semantic meaning from Vue templates</strong>
</p>

---

## Name Origin

**Croquis** (/kʁɔ.ki/) is a French term for a quick, sketchy drawing that captures the essential features of a subject. Artists use croquis to rapidly understand and convey the essence of a pose or scene.

In the art world, croquis:
- **Captures** - Quickly grasps the essential form
- **Analyzes** - Understands structure through rapid observation
- **Prepares** - Lays groundwork for detailed work

Similarly, `vize_croquis` provides:
- **Scope analysis** - Track variable scopes across templates
- **Binding resolution** - Resolve identifiers to declarations
- **Symbol tracking** - Fast lookup of bindings and metadata

## Features

- Hierarchical scope chain management
- Symbol table with reference tracking
- Support for Vue-specific scopes (v-for, v-slot)
- Unused symbol detection
- Mutation tracking

## Architecture

```
vize_armature (Parse)
       ↓
  vize_relief (AST)
       ↓
 vize_croquis (Semantic Analysis)  ◄── This crate
       ↓
vize_atelier_core (Transform)
```

## Usage

```rust
use vize_croquis::{ScopeChain, ScopeKind, ScopeBinding};
use vize_relief::BindingType;

let mut chain = ScopeChain::new();

// Add a ref binding at module level
chain.add_binding(
    "count".to_string(),
    ScopeBinding::new(BindingType::SetupRef, 0),
);

// Enter v-for scope
chain.enter_scope(ScopeKind::VFor);
chain.add_binding(
    "item".to_string(),
    ScopeBinding::new(BindingType::SetupConst, 100),
);

// Look up resolves through chain
assert!(chain.is_defined("count")); // from parent
assert!(chain.is_defined("item"));  // from current
```

## Part of the Vize Art Collection

`vize_croquis` is the semantic analysis layer of the Vize compiler's art-themed crate collection:

| Crate | Art Term | Role |
|-------|----------|------|
| vize_carton | Carton (Portfolio Case) | Shared utilities & allocator |
| vize_relief | Relief (Sculptured Surface) | AST definitions |
| vize_armature | Armature (Sculpture Frame) | Tokenizer & parser |
| **vize_croquis** | **Croquis (Quick Sketch)** | **Semantic analysis (this crate)** |
| vize_atelier_core | Atelier (Workshop) | Transforms & codegen |
| vize_atelier_sfc | Atelier (Workshop) | SFC compiler |
| vize_vitrine | Vitrine (Display Case) | Bindings (Node.js/WASM) |
| vize_canon | Canon (Standard) | Type checker |
| vize_glyph | Glyph (Letterform) | Formatter |
| vize_patina | Patina (Aged Surface) | Linter |

## License

MIT License
