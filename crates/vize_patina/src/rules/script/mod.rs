//! Script-level lint rules for Vue.js SFC files.
//!
//! These rules check TypeScript/JavaScript code in `<script>` and `<script setup>` blocks.
//! Rules in this module are **opt-in** and disabled by default.
//!
//! ## Enabling Script Rules
//!
//! Script rules can be enabled in your configuration:
//!
//! ```toml
//! [rules.script]
//! "prefer-import-from-vue" = "warn"
//! "no-internal-imports" = "error"
//! ```
//!
//! ## Vapor Mode Rules
//!
//! These rules help with Vapor mode compatibility (Vue 3.6+):
//!
//! - `script/no-options-api` - Disallow Options API patterns
//! - `script/no-get-current-instance` - Disallow getCurrentInstance() calls

mod no_async_in_computed;
mod no_deep_destructure_in_props;
mod no_get_current_instance;
mod no_import_compiler_macros;
mod no_internal_imports;
mod no_options_api;
mod no_reactive_destructure;
mod no_reserved_identifiers;
mod no_top_level_ref_in_script;
mod no_with_defaults;
mod prefer_computed;
mod prefer_import_from_vue;
mod prefer_ref_over_reactive;
mod prefer_use_attrs;
mod prefer_use_id;
mod prefer_use_slots;
mod prefer_use_template_ref;
mod require_function_return_type;
mod require_symbol_provide;

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

pub use no_async_in_computed::NoAsyncInComputed;
pub use no_deep_destructure_in_props::NoDeepDestructureInProps;
pub use no_get_current_instance::NoGetCurrentInstance;
pub use no_import_compiler_macros::NoImportCompilerMacros;
pub use no_internal_imports::NoInternalImports;
pub use no_options_api::NoOptionsApi;
pub use no_reactive_destructure::NoReactiveDestructure;
pub use no_reserved_identifiers::NoReservedIdentifiers;
pub use no_top_level_ref_in_script::NoTopLevelRefInScript;
pub use no_with_defaults::NoWithDefaults;
pub use prefer_computed::PreferComputed;
pub use prefer_import_from_vue::PreferImportFromVue;
pub use prefer_ref_over_reactive::PreferRefOverReactive;
pub use prefer_use_attrs::PreferUseAttrs;
pub use prefer_use_id::PreferUseId;
pub use prefer_use_slots::PreferUseSlots;
pub use prefer_use_template_ref::PreferUseTemplateRef;
pub use require_function_return_type::RequireFunctionReturnType;
pub use require_symbol_provide::RequireSymbolProvide;

/// Metadata for a script-level rule
pub struct ScriptRuleMeta {
    /// Rule name (e.g., "script/prefer-import-from-vue")
    pub name: &'static str,
    /// Rule description
    pub description: &'static str,
    /// Default severity (if enabled)
    pub default_severity: Severity,
}

/// Result of linting a script block
#[derive(Debug, Default)]
pub struct ScriptLintResult {
    pub diagnostics: Vec<LintDiagnostic>,
    pub error_count: usize,
    pub warning_count: usize,
}

impl ScriptLintResult {
    pub fn add_diagnostic(&mut self, diagnostic: LintDiagnostic) {
        match diagnostic.severity {
            Severity::Error => self.error_count += 1,
            Severity::Warning => self.warning_count += 1,
        }
        self.diagnostics.push(diagnostic);
    }

    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    pub fn has_warnings(&self) -> bool {
        self.warning_count > 0
    }
}

/// Trait for script-level lint rules
pub trait ScriptRule: Send + Sync {
    /// Get rule metadata
    fn meta(&self) -> &'static ScriptRuleMeta;

    /// Check the script content
    ///
    /// * `source` - The script block content
    /// * `offset` - The offset of the script block in the original file
    /// * `result` - Accumulator for diagnostics
    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult);
}

/// Linter for script blocks
pub struct ScriptLinter {
    rules: Vec<Box<dyn ScriptRule>>,
}

impl ScriptLinter {
    /// Create a new script linter with default rules (all disabled by default)
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Create a script linter with all available rules enabled
    pub fn with_all_rules() -> Self {
        Self {
            rules: vec![
                Box::new(PreferImportFromVue),
                Box::new(NoInternalImports),
                Box::new(NoOptionsApi),
                Box::new(NoGetCurrentInstance),
            ],
        }
    }

    /// Create a script linter with Vapor-specific rules enabled
    ///
    /// Includes rules that check for patterns not supported in Vapor mode:
    /// - `no-options-api` - Options API is not supported
    /// - `no-get-current-instance` - getCurrentInstance() returns null
    pub fn with_vapor_rules() -> Self {
        Self {
            rules: vec![Box::new(NoOptionsApi), Box::new(NoGetCurrentInstance)],
        }
    }

    /// Add a rule to the linter
    pub fn add_rule(&mut self, rule: Box<dyn ScriptRule>) {
        self.rules.push(rule);
    }

    /// Lint a script block
    pub fn lint(&self, source: &str, offset: usize) -> ScriptLintResult {
        let mut result = ScriptLintResult::default();

        for rule in &self.rules {
            rule.check(source, offset, &mut result);
        }

        result
    }

    /// Check if a script contains Vue imports (SIMD-accelerated)
    #[inline]
    pub fn has_vue_imports(source: &str) -> bool {
        let bytes = source.as_bytes();
        memmem::find(bytes, b"from 'vue'").is_some()
            || memmem::find(bytes, b"from \"vue\"").is_some()
            || memmem::find(bytes, b"from '@vue/").is_some()
            || memmem::find(bytes, b"from \"@vue/").is_some()
    }
}

impl Default for ScriptLinter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_vue_imports() {
        assert!(ScriptLinter::has_vue_imports("import { ref } from 'vue'"));
        assert!(ScriptLinter::has_vue_imports("import { ref } from \"vue\""));
        assert!(ScriptLinter::has_vue_imports(
            "import { h } from '@vue/runtime-core'"
        ));
        assert!(!ScriptLinter::has_vue_imports("import { foo } from 'bar'"));
    }

    #[test]
    fn test_empty_linter() {
        let linter = ScriptLinter::new();
        let result = linter.lint("import { ref } from 'vue'", 0);
        assert_eq!(result.error_count, 0);
        assert_eq!(result.warning_count, 0);
    }
}
