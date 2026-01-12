//! script/no-top-level-ref-in-script
//!
//! Disallow top-level ref/reactive in non-setup scripts to prevent Cross-Request State Pollution.
//!
//! In SSR (Server-Side Rendering) scenarios, top-level reactive state in regular
//! `<script>` blocks (not `<script setup>`) is shared across all requests.
//! This can lead to data leaking between different users' requests.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <script>
//! // This state is shared across all requests in SSR!
//! const count = ref(0)
//! const user = reactive({ name: '' })
//!
//! export default {
//!   setup() {
//!     return { count, user }
//!   }
//! }
//! </script>
//! ```
//!
//! ### Valid
//! ```vue
//! <script setup>
//! // Script setup creates fresh state per request
//! const count = ref(0)
//! </script>
//!
//! <script>
//! // Constants are fine
//! const API_URL = 'https://api.example.com'
//!
//! // Functions that create state are fine
//! function createState() {
//!   return reactive({ count: 0 })
//! }
//!
//! export default {
//!   setup() {
//!     // Create state inside setup
//!     const count = ref(0)
//!     return { count }
//!   }
//! }
//! </script>
//! ```

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/no-top-level-ref-in-script",
    description: "Disallow top-level ref/reactive to prevent Cross-Request State Pollution",
    default_severity: Severity::Error,
};

/// Prevent top-level reactive state in non-setup scripts
pub struct NoTopLevelRefInScript;

impl ScriptRule for NoTopLevelRefInScript {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // This rule is for regular <script> blocks, not <script setup>
        // The caller should only pass non-setup script content

        // Find top-level reactive state patterns
        // We need to ensure they're at module level (not inside functions/setup)

        let reactive_patterns = [
            ("ref(", "ref"),
            ("reactive(", "reactive"),
            ("computed(", "computed"),
            ("shallowRef(", "shallowRef"),
            ("shallowReactive(", "shallowReactive"),
        ];

        for (pattern, name) in reactive_patterns {
            self.check_top_level_usage(source, bytes, offset, pattern, name, result);
        }
    }
}

impl NoTopLevelRefInScript {
    fn check_top_level_usage(
        &self,
        source: &str,
        bytes: &[u8],
        offset: usize,
        pattern: &str,
        name: &str,
        result: &mut ScriptLintResult,
    ) {
        let finder = memmem::Finder::new(pattern.as_bytes());
        let mut search_start = 0;

        while let Some(pos) = finder.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + pattern.len();

            // Make sure it's not part of another identifier
            if abs_pos > 0 {
                let prev_char = bytes[abs_pos - 1];
                if prev_char.is_ascii_alphanumeric() || prev_char == b'_' {
                    continue;
                }
            }

            // Check if this is at module/top level by counting braces
            let before = &source[..abs_pos];

            // Count brace depth - if we're inside braces, it's not top-level
            let mut brace_depth: i32 = 0;
            let mut in_string = false;
            let mut string_char = ' ';

            for c in before.chars() {
                if in_string {
                    if c == string_char && !before.ends_with('\\') {
                        in_string = false;
                    }
                } else {
                    match c {
                        '"' | '\'' | '`' => {
                            in_string = true;
                            string_char = c;
                        }
                        '{' => brace_depth += 1,
                        '}' => brace_depth = brace_depth.saturating_sub(1),
                        _ => {}
                    }
                }
            }

            // Only report if at top level (brace_depth == 0)
            // Also check it's a variable assignment (const/let)
            if brace_depth == 0 {
                // Look backwards for const or let
                let line_start = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
                let line = before[line_start..].trim();

                if line.starts_with("const ") || line.starts_with("let ") {
                    result.add_diagnostic(
                        LintDiagnostic::error(
                            META.name,
                            format!(
                                "Top-level {}() in <script> can cause Cross-Request State Pollution in SSR",
                                name
                            ),
                            (offset + abs_pos) as u32,
                            (offset + abs_pos + pattern.len()) as u32,
                        )
                        .with_help(
                            "Move reactive state inside setup() or use <script setup>. \
                             Top-level state is shared across requests in SSR.",
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
        linter.add_rule(Box::new(NoTopLevelRefInScript));
        linter
    }

    #[test]
    fn test_valid_inside_setup() {
        let linter = create_linter();
        let result = linter.lint(
            r#"export default {
  setup() {
    const count = ref(0)
    return { count }
  }
}"#,
            0,
        );
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_inside_function() {
        let linter = create_linter();
        let result = linter.lint(
            r#"function createState() {
  return reactive({ count: 0 })
}"#,
            0,
        );
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_top_level_ref() {
        let linter = create_linter();
        let result = linter.lint("const count = ref(0)", 0);
        assert_eq!(result.error_count, 1);
        assert!(result.diagnostics[0]
            .message
            .contains("Cross-Request State Pollution"));
    }

    #[test]
    fn test_invalid_top_level_reactive() {
        let linter = create_linter();
        let result = linter.lint("const state = reactive({ count: 0 })", 0);
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_valid_const_string() {
        let linter = create_linter();
        let result = linter.lint("const API_URL = 'https://api.example.com'", 0);
        assert_eq!(result.error_count, 0);
    }
}
