//! script/prefer-use-attrs
//!
//! Recommend using useAttrs() over $attrs or context.attrs.
//!
//! In Composition API, useAttrs() is the preferred way to access
//! fallthrough attributes. It's more explicit and works in both
//! `<script setup>` and regular setup functions.
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! // In Options API style
//! export default {
//!   setup(props, { attrs }) {
//!     console.log(attrs.class)
//!   }
//! }
//!
//! // Accessing $attrs in template
//! <div :class="$attrs.class"></div>
//! ```
//!
//! ### Valid
//! ```ts
//! // Using useAttrs()
//! const attrs = useAttrs()
//! console.log(attrs.class)
//! ```

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/prefer-use-attrs",
    description: "Recommend using useAttrs() over context.attrs",
    default_severity: Severity::Warning,
};

/// Prefer useAttrs() rule
pub struct PreferUseAttrs;

impl ScriptRule for PreferUseAttrs {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // Check for destructuring { attrs } from setup context
        // Pattern: setup(props, { attrs }) or setup(_, { attrs })
        let finder = memmem::Finder::new(b"{ attrs }");
        let mut search_start = 0;

        while let Some(pos) = finder.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 9;

            // Check if this is in a setup function context
            let before = &source[..abs_pos];
            if before.contains("setup(") {
                result.add_diagnostic(
                    LintDiagnostic::warn(
                        META.name,
                        "Prefer useAttrs() over destructuring attrs from setup context",
                        (offset + abs_pos) as u32,
                        (offset + abs_pos + 9) as u32,
                    )
                    .with_help("Use `const attrs = useAttrs()` instead"),
                );
            }
        }

        // Check for context.attrs usage
        let finder2 = memmem::Finder::new(b"context.attrs");
        search_start = 0;

        while let Some(pos) = finder2.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 13;

            result.add_diagnostic(
                LintDiagnostic::warn(
                    META.name,
                    "Prefer useAttrs() over context.attrs",
                    (offset + abs_pos) as u32,
                    (offset + abs_pos + 13) as u32,
                )
                .with_help("Use `const attrs = useAttrs()` instead"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::script::ScriptLinter;

    fn create_linter() -> ScriptLinter {
        let mut linter = ScriptLinter::new();
        linter.add_rule(Box::new(PreferUseAttrs));
        linter
    }

    #[test]
    fn test_valid_use_attrs() {
        let linter = create_linter();
        let result = linter.lint("const attrs = useAttrs()", 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_context_attrs() {
        let linter = create_linter();
        let result = linter.lint("console.log(context.attrs)", 0);
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_destructure_attrs() {
        let linter = create_linter();
        let result = linter.lint("setup(props, { attrs }) {", 0);
        assert_eq!(result.warning_count, 1);
    }
}
