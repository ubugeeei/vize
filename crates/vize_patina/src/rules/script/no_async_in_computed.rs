//! script/no-async-in-computed
//!
//! Disallow async functions in computed properties.
//!
//! Computed properties must be synchronous and return a value immediately.
//! Using async functions or Promises in computed will cause unexpected behavior
//! since the computed will return a Promise object instead of the resolved value.
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! const data = computed(async () => {
//!   const response = await fetch('/api/data')
//!   return response.json()
//! })
//! ```
//!
//! ### Valid
//! ```ts
//! // Use ref + watchEffect for async operations
//! const data = ref(null)
//! watchEffect(async () => {
//!   const response = await fetch('/api/data')
//!   data.value = await response.json()
//! })
//!
//! // Or use a dedicated async state library
//! const { data } = useAsyncData(() => fetch('/api/data'))
//! ```

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/no-async-in-computed",
    description: "Disallow async functions in computed properties",
    default_severity: Severity::Error,
};

/// Disallow async in computed
pub struct NoAsyncInComputed;

impl ScriptRule for NoAsyncInComputed {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // Fast bailout: check if computed is used
        if memmem::find(bytes, b"computed").is_none() {
            return;
        }

        // Look for patterns like:
        // computed(async () => ...)
        // computed(async function() ...)

        let finder = memmem::Finder::new(b"computed(");
        let mut search_start = 0;

        while let Some(pos) = finder.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 9;

            // Get the content after "computed("
            let after = &source[abs_pos + 9..];
            let trimmed = after.trim_start();

            // Check if it starts with async
            if let Some(after_async) = trimmed.strip_prefix("async") {
                // Make sure "async" is followed by whitespace or arrow/function
                let next_char = after_async.chars().next();

                if matches!(next_char, Some(' ') | Some('\t') | Some('\n') | Some('(')) {
                    result.add_diagnostic(
                        LintDiagnostic::error(
                            META.name,
                            "Computed properties cannot be async. They must return a value synchronously.",
                            (offset + abs_pos) as u32,
                            (offset + abs_pos + 9 + trimmed.find("async").unwrap_or(0) + 5) as u32,
                        )
                        .with_help(
                            "Use ref with watchEffect for async operations: \
                             `const data = ref(null); watchEffect(async () => { data.value = await fetchData() })`",
                        ),
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
        linter.add_rule(Box::new(NoAsyncInComputed));
        linter
    }

    #[test]
    fn test_valid_sync_computed() {
        let linter = create_linter();
        let result = linter.lint("const doubled = computed(() => count.value * 2)", 0);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_async_arrow_computed() {
        let linter = create_linter();
        let result = linter.lint("const data = computed(async () => await fetch('/api'))", 0);
        assert_eq!(result.error_count, 1);
        assert!(result.diagnostics[0].message.contains("async"));
    }

    #[test]
    fn test_invalid_async_function_computed() {
        let linter = create_linter();
        let result = linter.lint(
            "const data = computed(async function() { return await fetch('/api') })",
            0,
        );
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_valid_watcheffect_async() {
        let linter = create_linter();
        let result = linter.lint(
            "watchEffect(async () => { data.value = await fetch('/api') })",
            0,
        );
        assert_eq!(result.error_count, 0);
    }
}
