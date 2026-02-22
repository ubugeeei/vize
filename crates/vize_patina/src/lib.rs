//! # vize_patina
//!
//! Patina - The quality checker for Vize.
//! Linter for Vue.js Single File Components.
//!
//! ## Name Origin
//!
//! **Patina** (/ˈpætɪnə/) refers to the greenish layer that forms on copper,
//! bronze, and similar metals through oxidation over time. In art and antiques,
//! patina is highly valued as it indicates authenticity, age, and quality.
//! `vize_patina` examines Vue SFC code to ensure its quality and authenticity.
//!
//! ## Features
//!
//! - Rich diagnostic output with code snippets and suggestions (like oxlint)
//! - eslint-plugin-vue compatible rules
//! - LSP-ready design for integration with vize_maestro
//!
//! ## Usage
//!
//! ```rust,ignore
//! use vize_patina::{Linter, OutputFormat, format_results};
//!
//! let linter = Linter::new();
//! let source = r#"<template><div v-for="item in items">{{ item }}</div></template>"#;
//! let result = linter.lint_template(source, "test.vue");
//!
//! if result.has_errors() {
//!     // Format and display errors
//!     let output = format_results(&[result], &[(filename.to_string(), source.to_string())], OutputFormat::Text);
//!     println!("{}", output);
//! }
//! ```
//!
//! ## Rules
//!
//! Currently implemented rules (eslint-plugin-vue compatible):
//!
//! ### Essential Rules
//! - `vue/require-v-for-key` - Require `v-bind:key` with `v-for` directives
//! - `vue/valid-v-for` - Enforce valid `v-for` directives
//! - `vue/valid-v-if` - Enforce valid `v-if` directives
//! - `vue/valid-v-else` - Enforce valid `v-else` directives
//! - `vue/valid-v-bind` - Enforce valid `v-bind` directives
//! - `vue/valid-v-on` - Enforce valid `v-on` directives
//! - `vue/valid-v-model` - Enforce valid `v-model` directives
//! - `vue/valid-v-show` - Enforce valid `v-show` directives
//! - `vue/no-use-v-if-with-v-for` - Disallow using `v-if` on the same element as `v-for`
//! - `vue/no-unused-vars` - Disallow unused variable definitions in `v-for` directives
//! - `vue/no-duplicate-attributes` - Disallow duplicate attributes
//! - `vue/no-template-key` - Disallow key attribute on `<template>`
//! - `vue/no-textarea-mustache` - Disallow mustache interpolation in `<textarea>`
//! - `vue/no-dupe-v-else-if` - Disallow duplicate conditions in v-if chains
//! - `vue/no-reserved-component-names` - Disallow reserved component names
//!
//! ### Strongly Recommended Rules
//! - `vue/no-template-shadow` - Disallow variable shadowing in v-for
//! - `vue/no-multi-spaces` - Disallow multiple consecutive spaces
//! - `vue/v-bind-style` - Enforce v-bind directive style (shorthand or longform)
//! - `vue/v-on-style` - Enforce v-on directive style (shorthand or longform)
//!
//! ### Vapor Migration Rules (based on Vue 3.6.0-beta.1)
//!
//! Template rules:
//! - `vapor/no-vue-lifecycle-events` - Disallow @vue:xxx per-element lifecycle events
//! - `vapor/no-suspense` - Warn about Suspense in Vapor-only apps
//! - `vapor/prefer-static-class` - Prefer static class over dynamic binding
//! - `vapor/no-inline-template` - Disallow deprecated inline-template
//!
//! Script rules (opt-in):
//! - `script/no-options-api` - Disallow Options API patterns (Vapor is Composition-only)
//! - `script/no-get-current-instance` - Disallow getCurrentInstance() (returns null in Vapor)
//!
//! ### Musea Rules (for *.art.vue files)
//! - `musea/require-title` - Require title attribute in `<art>` block
//! - `musea/require-component` - Require component attribute in `<art>` block
//! - `musea/valid-variant` - Require name attribute in `<variant>` blocks
//! - `musea/no-empty-variant` - Disallow empty variant blocks
//! - `musea/unique-variant-names` - Require unique variant names
//! - `musea/prefer-design-tokens` - Prefer design token CSS variables over hardcoded primitive values
//!
//! ### Script Rules (opt-in, default off)
//! - `script/prefer-import-from-vue` - Prefer importing from 'vue' instead of internal packages
//! - `script/no-internal-imports` - Disallow importing from Vue internal modules

mod context;
mod diagnostic;
mod linter;
pub mod output;
mod rule;
pub mod rules;
pub mod telegraph;
mod visitor;

pub use context::LintContext;
pub use diagnostic::{
    render_help, Fix, HelpLevel, HelpRenderTarget, LintDiagnostic, LintSummary, Severity, TextEdit,
};
pub use linter::{LintResult, Linter};
pub use output::{format_results, format_summary, OutputFormat};
pub use rule::{Rule, RuleCategory, RuleMeta, RuleRegistry};
pub use telegraph::{Emitter, JsonEmitter, LspDiagnostic, LspEmitter, Telegraph, TextEmitter};
pub use vize_carton::i18n::Locale;

/// Lint a Vue template source with default rules
///
/// This is a convenience function for simple use cases.
/// For more control, use `Linter::new()` directly.
pub fn lint(source: &str, filename: &str) -> LintResult {
    Linter::new().lint_template(source, filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lint_function() {
        let result = lint("<div v-for=\"item in items\"></div>", "test.vue");
        // Should have error for missing :key
        assert!(result.has_errors());
    }

    #[test]
    fn test_lint_valid_template() {
        let result = lint(
            "<div v-for=\"item in items\" :key=\"item.id\">{{ item }}</div>",
            "test.vue",
        );
        assert!(!result.has_errors());
    }
}
