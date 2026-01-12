//! script/require-function-return-type
//!
//! Require return type annotations on functions.
//!
//! Function definitions should declare their return type explicitly
//! to improve code readability and catch type errors early.
//! This applies to named functions, arrow functions assigned to const,
//! and methods in objects.
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! const add = (a: number, b: number) => {
//!   return a + b
//! }
//!
//! function greet(name: string) {
//!   return `Hello, ${name}`
//! }
//! ```
//!
//! ### Valid
//! ```ts
//! const add = (a: number, b: number): number => {
//!   return a + b
//! }
//!
//! function greet(name: string): string {
//!   return `Hello, ${name}`
//! }
//! ```
//!
//! ### Exceptions
//! - Callback functions passed as arguments (inferred from context)
//! - Arrow functions without block body (e.g., `x => x + 1`)

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/require-function-return-type",
    description: "Require return type annotations on functions",
    default_severity: Severity::Warning,
};

/// Require function return type annotations
pub struct RequireFunctionReturnType;

impl RequireFunctionReturnType {
    /// Check if the function signature has a return type
    fn has_return_type(signature_end: &str) -> bool {
        let trimmed = signature_end.trim_start();
        // After closing paren, should have : for return type
        trimmed.starts_with(':')
    }

    /// Find the function name from context
    fn extract_function_name(source: &str, pos: usize) -> Option<String> {
        // Look backwards for 'function ' or 'const '
        let before = &source[..pos];

        // Try to find function name
        if let Some(func_pos) = before.rfind("function ") {
            let after_func = &before[func_pos + 9..];
            let name: String = after_func
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }

        // Try to find const name
        if let Some(const_pos) = before.rfind("const ") {
            let after_const = &before[const_pos + 6..];
            let name: String = after_const
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }

        None
    }
}

impl ScriptRule for RequireFunctionReturnType {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // Fast bailout: check if there are any function definitions
        if memmem::find(bytes, b"function").is_none() && memmem::find(bytes, b"=>").is_none() {
            return;
        }

        // Find function declarations: function name(...)
        let func_finder = memmem::Finder::new(b"function ");
        let mut search_start = 0;

        while let Some(pos) = func_finder.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 9;

            // Find the closing parenthesis of parameters
            let rest = &source[abs_pos..];
            if let Some(paren_start) = rest.find('(') {
                // Find matching close paren
                let mut depth = 0;
                let mut close_pos = None;
                for (i, c) in rest[paren_start..].char_indices() {
                    match c {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                close_pos = Some(paren_start + i);
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(cp) = close_pos {
                    let after_paren = &rest[cp + 1..];
                    if !Self::has_return_type(after_paren) {
                        let func_name = Self::extract_function_name(source, abs_pos + paren_start)
                            .unwrap_or_else(|| "anonymous".to_string());

                        result.add_diagnostic(
                            LintDiagnostic::warn(
                                META.name,
                                format!(
                                    "Function \"{}\" is missing a return type annotation",
                                    func_name
                                ),
                                (offset + abs_pos) as u32,
                                (offset + abs_pos + cp + 1) as u32,
                            )
                            .with_help("Add a return type annotation: `function fn(...): ReturnType { ... }`"),
                        );
                    }
                }
            }
        }

        // Find arrow functions: const name = (...) =>
        let arrow_finder = memmem::Finder::new(b") =>");
        search_start = 0;

        while let Some(pos) = arrow_finder.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 4;

            // Check if there's a return type before ) =>
            // Look for ): pattern before the closing paren
            let before = &source[..abs_pos];

            // Find the matching opening paren
            let mut depth = 0;
            let mut open_pos = None;
            for (i, c) in before.chars().rev().enumerate() {
                match c {
                    ')' => depth += 1,
                    '(' => {
                        if depth == 0 {
                            open_pos = Some(before.len() - i - 1);
                            break;
                        }
                        depth -= 1;
                    }
                    _ => {}
                }
            }

            if let Some(op) = open_pos {
                // Check what's between open paren and closing ) =>
                let params_section = &source[op..abs_pos + 1];

                // A return type would look like ): Type, so after the last )
                // we should see : before =>
                if !params_section.contains("):") {
                    let func_name = Self::extract_function_name(source, op)
                        .unwrap_or_else(|| "anonymous".to_string());

                    // Skip if this looks like a callback (inside another function call)
                    let before_paren = &source[..op];
                    let is_callback = before_paren
                        .trim_end()
                        .chars()
                        .last()
                        .map(|c| c == ',' || c == '(')
                        .unwrap_or(false);

                    if !is_callback {
                        result.add_diagnostic(
                            LintDiagnostic::warn(
                                META.name,
                                format!(
                                    "Arrow function \"{}\" is missing a return type annotation",
                                    func_name
                                ),
                                (offset + op) as u32,
                                (offset + abs_pos + 4) as u32,
                            )
                            .with_help("Add a return type annotation: `const fn = (...): ReturnType => { ... }`"),
                        );
                    }
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
        linter.add_rule(Box::new(RequireFunctionReturnType));
        linter
    }

    #[test]
    fn test_valid_function_with_return_type() {
        let linter = create_linter();
        let result = linter.lint(
            "function greet(name: string): string { return `Hello, ${name}` }",
            0,
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_function_without_return_type() {
        let linter = create_linter();
        let result = linter.lint(
            "function greet(name: string) { return `Hello, ${name}` }",
            0,
        );
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].message.contains("greet"));
    }

    #[test]
    fn test_no_functions() {
        let linter = create_linter();
        let result = linter.lint("const x = 1\nconst y = 2", 0);
        assert_eq!(result.warning_count, 0);
    }
}
