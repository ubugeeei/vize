//! script/no-with-defaults
//!
//! Discourage use of withDefaults in favor of destructuring defaults.
//!
//! Since Vue 3.5, you can use JavaScript's native destructuring with default
//! values directly in defineProps. This is more concise and idiomatic.
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! // Using withDefaults (verbose)
//! const props = withDefaults(defineProps<{
//!   count?: number
//!   name?: string
//! }>(), {
//!   count: 0,
//!   name: 'default'
//! })
//! ```
//!
//! ### Valid
//! ```ts
//! // Using destructuring defaults (Vue 3.5+)
//! const { count = 0, name = 'default' } = defineProps<{
//!   count?: number
//!   name?: string
//! }>()
//!
//! // Or without destructuring if defaults not needed
//! const props = defineProps<{ count: number }>()
//! ```

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/no-with-defaults",
    description: "Discourage withDefaults in favor of destructuring defaults (Vue 3.5+)",
    default_severity: Severity::Warning,
};

/// Discourage withDefaults usage
pub struct NoWithDefaults;

impl ScriptRule for NoWithDefaults {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // Fast bailout
        if memmem::find(bytes, b"withDefaults").is_none() {
            return;
        }

        // Find all withDefaults calls
        let finder = memmem::Finder::new(b"withDefaults(");
        let mut search_start = 0;

        while let Some(pos) = finder.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 12;

            result.add_diagnostic(
                LintDiagnostic::warn(
                    META.name,
                    "Prefer destructuring defaults over withDefaults (Vue 3.5+)",
                    (offset + abs_pos) as u32,
                    (offset + abs_pos + 12) as u32,
                )
                .with_help(
                    "Use destructuring with defaults: \
                     `const { count = 0, name = 'default' } = defineProps<Props>()`",
                ),
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
        linter.add_rule(Box::new(NoWithDefaults));
        linter
    }

    #[test]
    fn test_valid_destructuring_defaults() {
        let linter = create_linter();
        let result = linter.lint("const { count = 0 } = defineProps<{ count?: number }>()", 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_with_defaults() {
        let linter = create_linter();
        let result = linter.lint(
            "const props = withDefaults(defineProps<{ count?: number }>(), { count: 0 })",
            0,
        );
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].message.contains("withDefaults"));
    }

    #[test]
    fn test_no_with_defaults() {
        let linter = create_linter();
        let result = linter.lint("const props = defineProps<{ name: string }>()", 0);
        assert_eq!(result.warning_count, 0);
    }
}
