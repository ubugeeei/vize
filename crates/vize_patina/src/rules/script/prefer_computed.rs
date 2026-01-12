//! script/prefer-computed
//!
//! Prefer computed properties over manually syncing reactive state.
//!
//! When a value can be derived from other reactive state, use `computed()`
//! instead of manually updating a separate `ref` with a watcher.
//!
//! This follows the principle: "reactive state that can be computed from
//! other state should use computed properties, not be actively defined."
//!
//! ## Examples
//!
//! ### Not Recommended
//! ```ts
//! const count = ref(0)
//! const doubled = ref(0)
//!
//! watch(count, (val) => {
//!   doubled.value = val * 2
//! })
//! ```
//!
//! ### Recommended
//! ```ts
//! const count = ref(0)
//! const doubled = computed(() => count.value * 2)
//! ```

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/prefer-computed",
    description: "Prefer computed() for derived reactive state",
    default_severity: Severity::Warning,
};

/// Prefer computed over watched refs
pub struct PreferComputed;

impl ScriptRule for PreferComputed {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // Fast bailout: check if there's a watch call
        if memmem::find(bytes, b"watch(").is_none() && memmem::find(bytes, b"watch (").is_none() {
            return;
        }

        // Look for patterns like:
        // watch(source, (val) => { target.value = ... })
        // This is a heuristic check - we look for .value = inside watch callbacks

        let watch_finder = memmem::Finder::new(b"watch(");
        let mut search_start = 0;

        while let Some(pos) = watch_finder.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 6;

            // Find the end of the watch call (heuristically by looking for closing })
            // This is a simplified check
            let rest = &source[abs_pos..];

            // Look for .value = pattern inside this watch
            if let Some(value_assign_pos) = rest.find(".value =") {
                // Check if it's within a reasonable distance (likely in the callback)
                if value_assign_pos < 200 {
                    // Likely a pattern where watch is used to sync derived state
                    result.add_diagnostic(
                        LintDiagnostic::warn(
                            META.name,
                            "Consider using computed() instead of watch() for derived state",
                            (offset + abs_pos) as u32,
                            (offset + abs_pos + 5) as u32,
                        )
                        .with_help(
                            "If the watch callback only assigns to a ref based on the watched value, \
                             use computed() instead: `const derived = computed(() => source.value * 2)`",
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
        linter.add_rule(Box::new(PreferComputed));
        linter
    }

    #[test]
    fn test_warn_watch_with_value_assignment() {
        let linter = create_linter();
        let result = linter.lint(
            r#"
const count = ref(0)
const doubled = ref(0)
watch(count, (val) => {
  doubled.value = val * 2
})
"#,
            0,
        );
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].message.contains("computed"));
    }

    #[test]
    fn test_valid_watch_without_value_assignment() {
        let linter = create_linter();
        let result = linter.lint(
            r#"
const count = ref(0)
watch(count, (val) => {
  console.log('count changed:', val)
})
"#,
            0,
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_no_watch() {
        let linter = create_linter();
        let result = linter.lint(
            r#"
const count = ref(0)
const doubled = computed(() => count.value * 2)
"#,
            0,
        );
        assert_eq!(result.warning_count, 0);
    }
}
