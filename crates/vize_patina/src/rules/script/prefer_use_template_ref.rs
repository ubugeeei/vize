//! script/prefer-use-template-ref
//!
//! Recommend useTemplateRef over ref for template references.
//!
//! Since Vue 3.5, useTemplateRef() is the recommended way to obtain template refs.
//! It provides better type inference and clearer intent compared to using
//! ref(null) with a matching template ref attribute.
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! // Old pattern (less clear intent)
//! const input = ref<HTMLInputElement | null>(null)
//! // <input ref="input" />
//!
//! // Using ref with null for template refs
//! const myElement = ref(null)
//! ```
//!
//! ### Valid
//! ```ts
//! // New pattern (Vue 3.5+)
//! const input = useTemplateRef<HTMLInputElement>('input')
//! // <input ref="input" />
//!
//! // Regular refs for reactive data (not template refs)
//! const count = ref(0)
//! const name = ref('hello')
//! ```

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/prefer-use-template-ref",
    description: "Recommend useTemplateRef over ref(null) for template references (Vue 3.5+)",
    default_severity: Severity::Warning,
};

/// Prefer useTemplateRef for template references
pub struct PreferUseTemplateRef;

impl ScriptRule for PreferUseTemplateRef {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // Fast bailout: check if ref is used
        if memmem::find(bytes, b"ref(").is_none() && memmem::find(bytes, b"ref<").is_none() {
            return;
        }

        // Look for patterns that suggest template refs:
        // 1. ref(null)
        // 2. ref<HTMLElement>(null)
        // 3. ref<SomeElement | null>(null)

        // Pattern 1: ref(null)
        let finder_null = memmem::Finder::new(b"ref(null)");
        let mut search_start = 0;

        while let Some(pos) = finder_null.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 9;

            // Make sure it's not part of another identifier like "toRef"
            if abs_pos > 0 {
                let prev_char = source.as_bytes()[abs_pos - 1];
                if prev_char.is_ascii_alphanumeric() || prev_char == b'_' {
                    continue;
                }
            }

            result.add_diagnostic(
                LintDiagnostic::warn(
                    META.name,
                    "Consider using useTemplateRef() for template references (Vue 3.5+)",
                    (offset + abs_pos) as u32,
                    (offset + abs_pos + 9) as u32,
                )
                .with_help(
                    "If this is a template ref, use: `const el = useTemplateRef<ElementType>('refName')`. \
                     If this is a regular ref that starts as null, you can ignore this warning.",
                ),
            );
        }

        // Pattern 2: ref<...Element...>(null) - suggests DOM element ref
        // Look for ref< followed by Element or HTML
        let finder_typed = memmem::Finder::new(b"ref<");
        search_start = 0;

        while let Some(pos) = finder_typed.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 4;

            // Make sure it's not part of another identifier
            if abs_pos > 0 {
                let prev_char = source.as_bytes()[abs_pos - 1];
                if prev_char.is_ascii_alphanumeric() || prev_char == b'_' {
                    continue;
                }
            }

            // Get the type parameter content
            let rest = &source[abs_pos + 4..];

            // Find closing > (accounting for nested generics)
            let mut depth = 1;
            let mut close_pos = None;
            for (i, c) in rest.char_indices() {
                match c {
                    '<' => depth += 1,
                    '>' => {
                        depth -= 1;
                        if depth == 0 {
                            close_pos = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if let Some(cp) = close_pos {
                let type_content = &rest[..cp];
                let after_type = &rest[cp + 1..];

                // Check if type suggests DOM element
                let is_element_type = type_content.contains("Element")
                    || type_content.contains("HTML")
                    || type_content.contains("SVG");

                // Check if initialized with null
                let starts_with_null = after_type.trim_start().starts_with("(null)");

                if is_element_type && starts_with_null {
                    let end_pos = abs_pos + 4 + cp + 1 + after_type.find(')').unwrap_or(5) + 1;

                    result.add_diagnostic(
                        LintDiagnostic::warn(
                            META.name,
                            "Use useTemplateRef() for DOM element references (Vue 3.5+)",
                            (offset + abs_pos) as u32,
                            (offset + end_pos) as u32,
                        )
                        .with_help(
                            "Replace with: `const el = useTemplateRef<ElementType>('refName')`",
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
        linter.add_rule(Box::new(PreferUseTemplateRef));
        linter
    }

    #[test]
    fn test_valid_use_template_ref() {
        let linter = create_linter();
        let result = linter.lint("const input = useTemplateRef<HTMLInputElement>('input')", 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_regular_ref() {
        let linter = create_linter();
        let result = linter.lint("const count = ref(0)", 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_ref_with_value() {
        let linter = create_linter();
        let result = linter.lint("const name = ref('hello')", 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_suspicious_ref_null() {
        let linter = create_linter();
        let result = linter.lint("const el = ref(null)", 0);
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_suspicious_element_ref() {
        let linter = create_linter();
        let result = linter.lint("const input = ref<HTMLInputElement | null>(null)", 0);
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].message.contains("useTemplateRef"));
    }

    #[test]
    fn test_toref_not_matched() {
        let linter = create_linter();
        let result = linter.lint("const x = toRef(state, 'count')", 0);
        assert_eq!(result.warning_count, 0);
    }
}
