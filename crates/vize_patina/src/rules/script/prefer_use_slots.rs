//! script/prefer-use-slots
//!
//! Recommend using useSlots() over $slots or context.slots.
//!
//! In Composition API, useSlots() is the preferred way to access
//! slot content programmatically. It's more explicit and works in both
//! `<script setup>` and regular setup functions.
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! // In Options API style
//! export default {
//!   setup(props, { slots }) {
//!     return () => h('div', slots.default?.())
//!   }
//! }
//!
//! // Using context.slots
//! const vnode = context.slots.default?.()
//! ```
//!
//! ### Valid
//! ```ts
//! // Using useSlots()
//! const slots = useSlots()
//! return () => h('div', slots.default?.())
//! ```

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/prefer-use-slots",
    description: "Recommend using useSlots() over context.slots",
    default_severity: Severity::Warning,
};

/// Prefer useSlots() rule
pub struct PreferUseSlots;

impl ScriptRule for PreferUseSlots {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // Check for destructuring { slots } from setup context
        let finder = memmem::Finder::new(b"{ slots }");
        let mut search_start = 0;

        while let Some(pos) = finder.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 9;

            let before = &source[..abs_pos];
            if before.contains("setup(") {
                result.add_diagnostic(
                    LintDiagnostic::warn(
                        META.name,
                        "Prefer useSlots() over destructuring slots from setup context",
                        (offset + abs_pos) as u32,
                        (offset + abs_pos + 9) as u32,
                    )
                    .with_help("Use `const slots = useSlots()` instead"),
                );
            }
        }

        // Check for context.slots usage
        let finder2 = memmem::Finder::new(b"context.slots");
        search_start = 0;

        while let Some(pos) = finder2.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 13;

            result.add_diagnostic(
                LintDiagnostic::warn(
                    META.name,
                    "Prefer useSlots() over context.slots",
                    (offset + abs_pos) as u32,
                    (offset + abs_pos + 13) as u32,
                )
                .with_help("Use `const slots = useSlots()` instead"),
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
        linter.add_rule(Box::new(PreferUseSlots));
        linter
    }

    #[test]
    fn test_valid_use_slots() {
        let linter = create_linter();
        let result = linter.lint("const slots = useSlots()", 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_context_slots() {
        let linter = create_linter();
        let result = linter.lint("return context.slots.default?.()", 0);
        assert_eq!(result.warning_count, 1);
    }
}
