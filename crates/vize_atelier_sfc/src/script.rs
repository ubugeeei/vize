//! Script block analysis and compilation.
//!
//! This module handles the compilation of `<script>` and `<script setup>` blocks,
//! including Compiler Macros like `defineProps`, `defineEmits`, etc.
//!
//! Module structure follows Vue.js official implementation.
//! Uses OXC for JavaScript/TypeScript parsing instead of Babel.

mod analyze_script_bindings;
mod context;
mod define_emits;
mod define_expose;
mod define_model;
mod define_options;
mod define_props;
mod define_props_destructure;
mod define_slots;
mod import_usage_check;
mod utils;

// Re-export main types
pub use analyze_script_bindings::{analyze_script_bindings, get_object_or_array_expression_keys};
pub use context::ScriptCompileContext;
pub use define_emits::{
    extract_runtime_emits, gen_runtime_emits, process_define_emits, DefineEmitsResult,
};
pub use define_props_destructure::{
    gen_props_access_exp, process_props_destructure, transform_destructured_props,
    PropsDestructureBinding, PropsDestructuredBindings,
};
pub use import_usage_check::{
    is_used_in_template, resolve_template_used_identifiers, resolve_template_v_model_identifiers,
    TemplateUsedIdentifiers,
};
pub use utils::{
    get_escaped_prop_name, is_compiler_macro_line, is_valid_identifier, MacroCall,
    ScriptSetupMacros,
};

// Re-export constants
pub use define_emits::DEFINE_EMITS;
pub use define_expose::DEFINE_EXPOSE;
pub use define_model::DEFINE_MODEL;
pub use define_options::DEFINE_OPTIONS;
pub use define_props::{DEFINE_PROPS, WITH_DEFAULTS};
pub use define_slots::DEFINE_SLOTS;

use crate::types::BindingMetadata;
use vize_croquis::analysis::Croquis as CroquisSummary;
use vize_croquis::script_parser::ScriptParseResult;

/// Analyze script setup and extract bindings
pub fn analyze_script_setup(content: &str) -> BindingMetadata {
    let mut ctx = ScriptCompileContext::new(content);
    ctx.analyze();
    ctx.bindings
}

/// Extract macro calls from script setup
pub fn extract_macros(content: &str) -> ScriptSetupMacros {
    let mut ctx = ScriptCompileContext::new(content);
    ctx.extract_all_macros();
    ctx.macros
}

// =============================================================================
// vize_croquis Integration
// =============================================================================

/// Fast script setup analysis using vize_croquis OXC parser.
///
/// This provides a high-performance analysis path that returns
/// a `ScriptParseResult` directly from vize_croquis.
///
/// Use this for:
/// - Quick analysis in linter
/// - Playground/editor integrations
/// - When full macro transformation is not needed
///
/// For full compilation with macro transformations, use `ScriptCompileContext`.
#[inline]
pub fn analyze_script_setup_fast(content: &str) -> ScriptParseResult {
    vize_croquis::script_parser::parse_script_setup(content)
}

/// Analyze script setup and return a croquis Croquis.
///
/// This uses vize_croquis for the core analysis and converts
/// the result to the shared Croquis format.
pub fn analyze_script_setup_to_summary(content: &str) -> CroquisSummary {
    let result = vize_croquis::script_parser::parse_script_setup(content);

    let mut summary = CroquisSummary::new();

    // Copy bindings
    summary.bindings = result.bindings;

    // Copy macros
    summary.macros = result.macros;

    // Copy reactivity
    summary.reactivity = result.reactivity;

    // Copy exports
    summary.type_exports = result.type_exports;
    summary.invalid_exports = result.invalid_exports;

    summary
}

/// Convert a full ScriptCompileContext analysis to Croquis.
///
/// This uses the full atelier_sfc analysis (which includes more detailed
/// type resolution) and converts to the shared format.
#[inline]
pub fn analyze_script_setup_full(content: &str) -> CroquisSummary {
    let mut ctx = ScriptCompileContext::new(content);
    ctx.analyze();
    ctx.to_analysis_summary()
}
