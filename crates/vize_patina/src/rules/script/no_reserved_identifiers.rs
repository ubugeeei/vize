//! script/no-reserved-identifiers
//!
//! Disallow using Vue compiler reserved identifiers.
//!
//! Vue's compiler generates internal variables with specific prefixes like
//! `__props`, `__emit`, `__sfc__`, etc. Using these identifiers in your
//! code can cause conflicts and unexpected behavior.
//!
//! ## Reserved Identifiers
//!
//! - `__props` - Internal props reference
//! - `__emit` - Internal emit function
//! - `__expose` - Internal expose function
//! - `__sfc__` - SFC metadata
//! - `__sfc_main` - Main component export
//! - `_ctx` - Render context
//! - `_cache` - Render cache
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! const __props = { name: 'test' }
//! const __emit = () => {}
//! let __sfc__ = {}
//! ```
//!
//! ### Valid
//! ```ts
//! const props = defineProps<Props>()
//! const emit = defineEmits<Emits>()
//! const myData = {}
//! ```

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/no-reserved-identifiers",
    description: "Disallow using Vue compiler reserved identifiers",
    default_severity: Severity::Error,
};

/// Reserved identifiers used by Vue compiler
const RESERVED_IDENTIFIERS: &[&str] = &[
    "__props",
    "__emit",
    "__expose",
    "__sfc__",
    "__sfc_main",
    "__injectCSSVars__",
    "_ctx",
    "_cache",
    "_setupState",
    "_instance",
    "_hoisted_",
    "_createBlock",
    "_createVNode",
    "_createElementVNode",
    "_resolveComponent",
    "_resolveDirective",
    "_withCtx",
    "_openBlock",
];

/// No reserved identifiers rule
pub struct NoReservedIdentifiers;

impl ScriptRule for NoReservedIdentifiers {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        for reserved in RESERVED_IDENTIFIERS {
            let finder = memmem::Finder::new(reserved.as_bytes());
            let mut search_start = 0;

            while let Some(pos) = finder.find(&bytes[search_start..]) {
                let abs_pos = search_start + pos;
                search_start = abs_pos + reserved.len();

                // Check if this is a variable declaration
                let before = &source[..abs_pos];
                let trimmed = before.trim_end();

                // Look for const, let, var, or function before the identifier
                let is_declaration = trimmed.ends_with("const")
                    || trimmed.ends_with("let")
                    || trimmed.ends_with("var")
                    || trimmed.ends_with("function");

                // Also check if it's being assigned
                let after = &source[abs_pos + reserved.len()..];
                let after_trimmed = after.trim_start();
                let is_assignment =
                    after_trimmed.starts_with('=') && !after_trimmed.starts_with("==");

                if is_declaration || is_assignment {
                    result.add_diagnostic(
                        LintDiagnostic::error(
                            META.name,
                            format!(
                                "'{}' is a Vue compiler reserved identifier and should not be used",
                                reserved
                            ),
                            (offset + abs_pos) as u32,
                            (offset + abs_pos + reserved.len()) as u32,
                        )
                        .with_help("Choose a different variable name to avoid conflicts with Vue internals"),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::script::ScriptLinter;

    fn create_linter() -> ScriptLinter {
        let mut linter = ScriptLinter::new();
        linter.add_rule(Box::new(NoReservedIdentifiers));
        linter
    }

    #[test]
    fn test_valid_normal_identifier() {
        let linter = create_linter();
        let result = linter.lint("const props = defineProps()", 0);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_reserved_props() {
        let linter = create_linter();
        let result = linter.lint("const __props = {}", 0);
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_invalid_reserved_emit() {
        let linter = create_linter();
        let result = linter.lint("let __emit = () => {}", 0);
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_invalid_reserved_sfc() {
        let linter = create_linter();
        let result = linter.lint("var __sfc__ = {}", 0);
        assert_eq!(result.error_count, 1);
    }
}
