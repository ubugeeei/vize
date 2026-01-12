//! script/require-symbol-provide
//!
//! Recommend using Symbol as injection key for provide/inject.
//!
//! Using Symbol keys for provide/inject avoids naming collisions and makes
//! the dependency injection more explicit and type-safe.
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! // String keys can collide
//! provide('user', user)
//! const user = inject('user')
//!
//! // Magic strings are error-prone
//! provide('theme', { dark: true })
//! ```
//!
//! ### Valid
//! ```ts
//! // Define injection key with Symbol
//! export const UserKey: InjectionKey<User> = Symbol('user')
//!
//! // Provide with Symbol
//! provide(UserKey, user)
//!
//! // Inject with Symbol
//! const user = inject(UserKey)
//! ```

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/require-symbol-provide",
    description: "Recommend using Symbol as injection key for provide/inject",
    default_severity: Severity::Warning,
};

/// Require Symbol for provide/inject keys
pub struct RequireSymbolProvide;

impl ScriptRule for RequireSymbolProvide {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // Check for provide() with string literal
        self.check_call(source, bytes, offset, "provide(", result);

        // Check for inject() with string literal
        self.check_call(source, bytes, offset, "inject(", result);
    }
}

impl RequireSymbolProvide {
    fn check_call(
        &self,
        source: &str,
        bytes: &[u8],
        offset: usize,
        pattern: &str,
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

            // Check if the first argument is a string literal
            let after = &source[abs_pos + pattern.len()..];
            let trimmed = after.trim_start();

            if trimmed.starts_with('\'') || trimmed.starts_with('"') || trimmed.starts_with('`') {
                let fn_name = if pattern == "provide(" {
                    "provide"
                } else {
                    "inject"
                };

                result.add_diagnostic(
                    LintDiagnostic::warn(
                        META.name,
                        format!(
                            "Consider using a Symbol key with {} instead of a string literal",
                            fn_name
                        ),
                        (offset + abs_pos) as u32,
                        (offset + abs_pos + pattern.len()) as u32,
                    )
                    .with_help(
                        "Define an InjectionKey with Symbol: \
                         `export const MyKey: InjectionKey<MyType> = Symbol('myKey')`",
                    ),
                );
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
        linter.add_rule(Box::new(RequireSymbolProvide));
        linter
    }

    #[test]
    fn test_valid_symbol_provide() {
        let linter = create_linter();
        let result = linter.lint("provide(UserKey, user)", 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_symbol_inject() {
        let linter = create_linter();
        let result = linter.lint("const user = inject(UserKey)", 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_string_provide() {
        let linter = create_linter();
        let result = linter.lint("provide('user', userData)", 0);
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].message.contains("Symbol"));
    }

    #[test]
    fn test_invalid_string_inject() {
        let linter = create_linter();
        let result = linter.lint("const user = inject('user')", 0);
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_template_literal() {
        let linter = create_linter();
        let result = linter.lint("provide(`theme`, theme)", 0);
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_no_provide_inject() {
        let linter = create_linter();
        let result = linter.lint("const x = ref(0)", 0);
        assert_eq!(result.warning_count, 0);
    }
}
