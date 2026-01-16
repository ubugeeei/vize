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

// Core modules
mod scope;
mod symbol;

// Analysis modules
pub mod analysis;
pub mod analyzer;
pub mod builtins;
pub mod call_graph;
pub mod css;
pub mod display;
pub mod hoist;
pub mod import_resolver;
pub mod macros;
pub mod naming;
pub mod optimization;
pub mod provide;
pub mod reactivity;
pub mod reactivity_tracking;
pub mod script_parser;
pub mod types;
pub mod virtual_ts;

// Cross-file analysis (opt-in)
pub mod cross_file;

// Re-export commonly used utilities from vize_carton for convenience
pub use vize_carton::{
    is_builtin_directive, is_builtin_tag, is_html_tag, is_math_ml_tag, is_native_tag,
    is_reserved_prop, is_svg_tag, is_void_tag,
};

// Re-export core types
pub use scope::*;
pub use symbol::*;

// Re-export analysis types
pub use analysis::{
    AnalysisStats, BindingMetadata, Croquis, InvalidExport, InvalidExportKind, TemplateExpression,
    TemplateExpressionKind, TypeExport, TypeExportKind, UndefinedRef, UnusedTemplateVar,
    UnusedVarContext,
};
pub use analyzer::{Analyzer, AnalyzerOptions};

// Re-export common types
pub use vize_relief::BindingType;
