//! # Vize
//!
//! High-performance Vue.js toolchain written in Rust.
//!
//! This crate re-exports all Vize sub-crates for unified documentation.
//!
//! ## Crates
//!
//! - [`carton`] - Box/container utilities for memory management
//! - [`relief`] - Source text and span handling
//! - [`armature`] - Vue template AST definitions
//! - [`atelier_core`] - Core template compiler infrastructure
//! - [`atelier_dom`] - DOM mode template compiler
//! - [`atelier_vapor`] - Vapor mode template compiler
//! - [`atelier_sfc`] - Single File Component (SFC) parser and compiler
//! - [`glyph`] - TypeScript/JavaScript transformer
//! - [`patina`] - CSS/style processing with scoped styles
//! - [`canon`] - Code formatter
//! - [`musea`] - Multi-file project handling
//! - [`maestro`] - Language Server Protocol (LSP) implementation

/// Box/container utilities for memory management.
pub use vize_carton as carton;

/// Source text and span handling.
pub use vize_relief as relief;

/// Vue template AST definitions.
pub use vize_armature as armature;

/// Core template compiler infrastructure.
pub use vize_atelier_core as atelier_core;

/// DOM mode template compiler.
pub use vize_atelier_dom as atelier_dom;

/// Vapor mode template compiler.
pub use vize_atelier_vapor as atelier_vapor;

/// Single File Component (SFC) parser and compiler.
pub use vize_atelier_sfc as atelier_sfc;

/// TypeScript/JavaScript transformer.
#[cfg(feature = "glyph")]
pub use vize_glyph as glyph;

/// CSS/style processing with scoped styles.
pub use vize_patina as patina;

/// Code formatter.
pub use vize_canon as canon;

/// Multi-file project handling.
pub use vize_musea as musea;

/// Language Server Protocol (LSP) implementation.
pub use vize_maestro as maestro;
