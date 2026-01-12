//! # vize_croquis
//!
//! Croquis - The semantic analysis layer for Vize.
//!
//! ## Name Origin
//!
//! **Croquis** (/kʁɔ.ki/) is a French term for a quick, sketchy drawing that captures
//! the essential features of a subject. Like how artists use croquis to rapidly
//! capture the essence of a pose or scene, `vize_croquis` quickly analyzes Vue
//! templates to extract semantic meaning from the syntactic structure.
//!
//! ## Purpose
//!
//! This crate bridges the gap between parsing (vize_armature) and transformation
//! (vize_atelier_core) by providing:
//!
//! - **Scope Analysis**: Track variable scopes across templates and scripts
//! - **Binding Resolution**: Resolve identifiers to their declarations
//! - **Reactivity Tracking**: Understand ref/reactive dependencies
//! - **Symbol Tables**: Fast lookup of bindings and their metadata
//!
//! ## Architecture
//!
//! ```text
//! vize_armature (Parse)
//!        ↓
//!   vize_relief (AST)
//!        ↓
//!  vize_croquis (Semantic Analysis)  ← This crate
//!        ↓
//! vize_atelier_core (Transform)
//! ```

mod scope;
mod symbol;

pub use scope::*;
pub use symbol::*;

/// Re-export common types
pub use vize_relief::BindingType;
