//! script/prefer-use-id
//!
//! Recommend using useId() for generating unique IDs.
//!
//! useId() generates unique IDs that are stable across server and client
//! rendering, making it ideal for accessibility attributes and form elements.
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! // Manual ID generation (not SSR-safe)
//! const id = `input-${Math.random()}`
//! const id = `field-${Date.now()}`
//! let counter = 0; const id = `el-${counter++}`
//! ```
//!
//! ### Valid
//! ```ts
//! // Using useId() (Vue 3.5+)
//! const id = useId()
//!
//! // In template
//! <label :for="id">Name</label>
//! <input :id="id" />
//! ```
//!
//! ## Benefits
//!
//! - SSR-safe: Same ID on server and client
//! - Unique: No collisions between component instances
//! - Accessible: Perfect for aria-labelledby, aria-describedby

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/prefer-use-id",
    description: "Recommend using useId() for generating unique IDs (Vue 3.5+)",
    default_severity: Severity::Warning,
};

/// Prefer useId() rule
pub struct PreferUseId;

impl ScriptRule for PreferUseId {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // Check for Math.random() in ID generation context
        let patterns = [
            (b"Math.random()" as &[u8], "Math.random()"),
            (b"Date.now()" as &[u8], "Date.now()"),
            (b"crypto.randomUUID()" as &[u8], "crypto.randomUUID()"),
        ];

        for (pattern, name) in patterns {
            let finder = memmem::Finder::new(pattern);
            let mut search_start = 0;

            while let Some(pos) = finder.find(&bytes[search_start..]) {
                let abs_pos = search_start + pos;
                search_start = abs_pos + pattern.len();

                // Check if this looks like ID generation (has id or Id nearby)
                let line_start = source[..abs_pos].rfind('\n').map(|p| p + 1).unwrap_or(0);
                let line_end = source[abs_pos..]
                    .find('\n')
                    .map(|p| abs_pos + p)
                    .unwrap_or(source.len());
                let line = &source[line_start..line_end];

                let looks_like_id = line.to_lowercase().contains("id")
                    || line.contains("uuid")
                    || line.contains("unique");

                if looks_like_id {
                    result.add_diagnostic(
                        LintDiagnostic::warn(
                            META.name,
                            format!(
                                "Consider using useId() instead of {} for generating IDs (Vue 3.5+)",
                                name
                            ),
                            (offset + abs_pos) as u32,
                            (offset + abs_pos + pattern.len()) as u32,
                        )
                        .with_help(
                            "useId() provides SSR-safe, unique IDs: `const id = useId()`",
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
        linter.add_rule(Box::new(PreferUseId));
        linter
    }

    #[test]
    fn test_valid_use_id() {
        let linter = create_linter();
        let result = linter.lint("const id = useId()", 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_warns_math_random_id() {
        let linter = create_linter();
        let result = linter.lint("const id = `input-${Math.random()}`", 0);
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_warns_date_now_id() {
        let linter = create_linter();
        let result = linter.lint("const uniqueId = `el-${Date.now()}`", 0);
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_no_warn_random_not_id() {
        let linter = create_linter();
        let result = linter.lint("const value = Math.random() * 100", 0);
        assert_eq!(result.warning_count, 0);
    }
}
