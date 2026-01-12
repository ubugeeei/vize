//! script/no-reactive-destructure
//!
//! Disallow destructuring reactive objects which loses reactivity.
//!
//! When you destructure a reactive object, the resulting variables lose
//! their reactive connection. Changes to the original object won't be
//! reflected in the destructured variables.
//!
//! ## Examples
//!
//! ### Invalid
//! ```ts
//! const state = reactive({ count: 0, name: 'foo' })
//! const { count, name } = state  // loses reactivity!
//!
//! // Also passing reactive directly to functions that expect refs
//! someFunction(state.count)  // loses reactivity
//! ```
//!
//! ### Valid
//! ```ts
//! const state = reactive({ count: 0, name: 'foo' })
//!
//! // Use toRef or toRefs to maintain reactivity
//! const count = toRef(state, 'count')
//! const { count, name } = toRefs(state)
//!
//! // Or use computed for derived values
//! const doubleCount = computed(() => state.count * 2)
//!
//! // Pass refs or computed to functions
//! someFunction(toRef(state, 'count'))
//! ```

use memchr::memmem;

use crate::diagnostic::{LintDiagnostic, Severity};

use super::{ScriptLintResult, ScriptRule, ScriptRuleMeta};

static META: ScriptRuleMeta = ScriptRuleMeta {
    name: "script/no-reactive-destructure",
    description: "Disallow destructuring reactive objects which loses reactivity",
    default_severity: Severity::Warning,
};

/// Disallow reactive destructuring
pub struct NoReactiveDestructure;

impl NoReactiveDestructure {
    /// Track variable names that hold reactive() results
    fn find_reactive_vars(source: &str) -> Vec<String> {
        let mut vars = Vec::new();
        let bytes = source.as_bytes();

        // Find patterns like: const state = reactive(...)
        // or: let state = reactive(...)
        let finder = memmem::Finder::new(b"reactive(");
        let mut search_start = 0;

        while let Some(pos) = finder.find(&bytes[search_start..]) {
            let abs_pos = search_start + pos;
            search_start = abs_pos + 9;

            // Look backwards for variable name
            let before = &source[..abs_pos];

            // Find = before reactive(
            if let Some(eq_pos) = before.rfind('=') {
                let var_part = before[..eq_pos].trim_end();

                // Find const or let
                if let Some(decl_pos) = var_part.rfind("const ").or_else(|| var_part.rfind("let "))
                {
                    let is_const = var_part[decl_pos..].starts_with("const ");
                    let offset = if is_const { 6 } else { 4 };
                    let var_name: String = var_part[decl_pos + offset..]
                        .trim()
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();

                    if !var_name.is_empty() {
                        vars.push(var_name);
                    }
                }
            }
        }

        vars
    }
}

impl ScriptRule for NoReactiveDestructure {
    fn meta(&self) -> &'static ScriptRuleMeta {
        &META
    }

    fn check(&self, source: &str, offset: usize, result: &mut ScriptLintResult) {
        let bytes = source.as_bytes();

        // Fast bailout: check if reactive is used
        if memmem::find(bytes, b"reactive(").is_none() {
            return;
        }

        // Find all reactive variable names
        let reactive_vars = Self::find_reactive_vars(source);

        if reactive_vars.is_empty() {
            return;
        }

        // Look for destructuring of these variables
        // Pattern: const { ... } = varName or let { ... } = varName
        for var in &reactive_vars {
            let pattern = format!("}} = {}", var);
            let pattern_bytes = pattern.as_bytes();

            let finder = memmem::Finder::new(pattern_bytes);
            let mut search_start = 0;

            while let Some(pos) = finder.find(&bytes[search_start..]) {
                let abs_pos = search_start + pos;
                search_start = abs_pos + pattern.len();

                // Check if this is a destructuring (look back for {)
                let before = &source[..abs_pos + 1];
                if let Some(open_brace) = before.rfind('{') {
                    // Check this is a const/let declaration
                    let decl_part = &source[..open_brace];
                    if decl_part.trim_end().ends_with("const")
                        || decl_part.trim_end().ends_with("let")
                    {
                        result.add_diagnostic(
                            LintDiagnostic::warn(
                                META.name,
                                format!(
                                    "Destructuring reactive object \"{}\" loses reactivity",
                                    var
                                ),
                                (offset + open_brace) as u32,
                                (offset + abs_pos + pattern.len()) as u32,
                            )
                            .with_help(
                                "Use toRefs() to maintain reactivity: `const {{ a, b }} = toRefs({})` \
                                 or access properties directly: `{}.property`",
                            ),
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
        linter.add_rule(Box::new(NoReactiveDestructure));
        linter
    }

    #[test]
    fn test_valid_reactive_access() {
        let linter = create_linter();
        let result = linter.lint(
            "const state = reactive({ count: 0 })\nconst x = state.count",
            0,
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_torefs() {
        let linter = create_linter();
        let result = linter.lint(
            "const state = reactive({ count: 0 })\nconst { count } = toRefs(state)",
            0,
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_destructure() {
        let linter = create_linter();
        let result = linter.lint(
            "const state = reactive({ count: 0, name: 'foo' })\nconst { count, name } = state",
            0,
        );
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].message.contains("loses reactivity"));
    }

    #[test]
    fn test_no_reactive() {
        let linter = create_linter();
        let result = linter.lint("const x = ref(0)", 0);
        assert_eq!(result.warning_count, 0);
    }
}
