//! script/no-deep-destructure-in-props
//!
//! Disallow deeply nested destructuring in defineProps.
//!
//! Deep destructuring patterns like `const { a: { b = 1 }} = defineProps()`
//! are hard to read, prone to runtime errors, and make it difficult to
//! understand the prop structure at a glance.
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! // Deep nested destructuring
//! const { user: { name, age } } = defineProps<{ user: User }>()
//!
//! // Very deep nesting
//! const { config: { settings: { theme } } } = defineProps()
//! ```
//!
//! ### Valid
//! ```ts
//! // Simple destructuring (one level)
//! const { name, count = 0 } = defineProps<{ name: string; count?: number }>()
//!
//! // Access nested properties in the component instead
//! const props = defineProps<{ user: User }>()
//! const userName = computed(() => props.user.name)
//! ```

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/no-deep-destructure-in-props",
    description: "Disallow deeply nested destructuring in defineProps",
    default_severity: Severity::Warning,
};

/// Disallow deep destructuring in defineProps
pub struct NoDeepDestructureInProps {
    /// Maximum allowed nesting depth (default: 1)
    pub max_depth: usize,
}

impl Default for NoDeepDestructureInProps {
    fn default() -> Self {
        Self { max_depth: 1 }
    }
}

impl NoDeepDestructureInProps {
    /// Check if a destructuring pattern has nested objects beyond max_depth
    fn has_deep_nesting(pattern: &str, max_depth: usize) -> bool {
        let mut depth: usize = 0;
        let mut max_seen: usize = 0;

        for c in pattern.chars() {
            match c {
                '{' => {
                    depth += 1;
                    max_seen = max_seen.max(depth);
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        // max_seen > max_depth means we have deeper nesting than allowed
        // For max_depth = 1, we allow { a, b } but not { a: { b } }
        max_seen > max_depth
    }

    /// Extract the destructuring pattern from a defineProps call
    fn extract_destructure_pattern(source: &str, define_props_pos: usize) -> Option<&str> {
        // Look backwards from defineProps to find the pattern
        let before = &source[..define_props_pos];

        // Find the last '=' before defineProps
        let eq_pos = before.rfind('=')?;

        // Find 'const' or 'let' before that
        let decl_start = before[..eq_pos]
            .rfind("const ")
            .or_else(|| before[..eq_pos].rfind("let "))?;

        // Extract the pattern between const/let and =
        let pattern_start = if before[decl_start..].starts_with("const ") {
            decl_start + 6
        } else {
            decl_start + 4
        };

        let pattern = before[pattern_start..eq_pos].trim();

        // Only interested in destructuring patterns (starts with {)
        if pattern.starts_with('{') {
            Some(pattern)
        } else {
            None
        }
    }
}

impl ScriptRule for NoDeepDestructureInProps {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // Fast bailout: check if defineProps is used
        if memmem::find(bytes, b"defineProps").is_none() {
            return;
        }

        // Find all occurrences of defineProps
        let finder = memmem::Finder::new(b"defineProps");
        let mut search_start = 0;

        while let Some(pos) = finder.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 11;

            // Extract the destructuring pattern
            if let Some(pattern) = Self::extract_destructure_pattern(source, abs_pos) {
                if Self::has_deep_nesting(pattern, self.max_depth) {
                    // Find pattern position
                    let pattern_start = source[..abs_pos].rfind(pattern).unwrap_or(abs_pos);

                    result.add_diagnostic(
                        LintDiagnostic::warn(
                            META.name,
                            format!(
                                "Avoid deeply nested destructuring in defineProps (max depth: {})",
                                self.max_depth
                            ),
                            (offset + pattern_start) as u32,
                            (offset + pattern_start + pattern.len()) as u32,
                        )
                        .with_help(
                            "Use simple destructuring and access nested properties via computed or direct prop access: \
                             `const props = defineProps<...>(); const nested = computed(() => props.parent.child)`",
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
        linter.add_rule(Box::new(NoDeepDestructureInProps::default()));
        linter
    }

    #[test]
    fn test_valid_simple_destructure() {
        let linter = create_linter();
        let result = linter.lint(
            "const { name, count = 0 } = defineProps<{ name: string }>()",
            0,
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_no_destructure() {
        let linter = create_linter();
        let result = linter.lint("const props = defineProps<{ name: string }>()", 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_deep_destructure() {
        let linter = create_linter();
        let result = linter.lint(
            "const { user: { name } } = defineProps<{ user: User }>()",
            0,
        );
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].message.contains("deeply nested"));
    }

    #[test]
    fn test_invalid_very_deep_destructure() {
        let linter = create_linter();
        let result = linter.lint(
            "const { config: { settings: { theme } } } = defineProps()",
            0,
        );
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_has_deep_nesting() {
        assert!(!NoDeepDestructureInProps::has_deep_nesting("{ a, b }", 1));
        assert!(!NoDeepDestructureInProps::has_deep_nesting(
            "{ a, b = 1 }",
            1
        ));
        assert!(NoDeepDestructureInProps::has_deep_nesting(
            "{ a: { b } }",
            1
        ));
        assert!(NoDeepDestructureInProps::has_deep_nesting(
            "{ a: { b: { c } } }",
            1
        ));
        assert!(NoDeepDestructureInProps::has_deep_nesting(
            "{ a: { b: { c } } }",
            2
        ));
        assert!(!NoDeepDestructureInProps::has_deep_nesting(
            "{ a: { b } }",
            2
        ));
    }
}
