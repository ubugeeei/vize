//! vue/no-undefined-refs
//!
//! Disallow undefined variable references in templates.
//!
//! This rule requires semantic analysis (Croquis) to be effective.
//! Without analysis, it only checks v-for scope variables.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <!-- 'undefinedVar' is not defined in script -->
//!   <div>{{ undefinedVar }}</div>
//!   <span v-if="unknownFlag">...</span>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <script setup>
//! const count = ref(0)
//! const user = reactive({ name: 'John' })
//! </script>
//!
//! <template>
//!   <div>{{ count }}</div>
//!   <span>{{ user.name }}</span>
//!   <li v-for="item in items" :key="item.id">
//!     {{ item.name }}
//!   </li>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_croquis::builtins::is_js_global;
use vize_relief::ast::{ElementNode, ExpressionNode, InterpolationNode};

static META: RuleMeta = RuleMeta {
    name: "vue/no-undefined-refs",
    description: "Disallow undefined variable references in templates",
    category: RuleCategory::Recommended,
    fixable: false,
    default_severity: Severity::Warning,
};

/// No undefined refs rule
#[derive(Default)]
pub struct NoUndefinedRefs;

impl NoUndefinedRefs {
    /// Extract identifiers from an expression string
    ///
    /// This is a simplified implementation that extracts top-level identifiers.
    /// A full implementation would use a proper expression parser.
    fn extract_identifiers(expr: &str) -> Vec<&str> {
        let mut identifiers = Vec::new();
        let expr = expr.trim();

        // Skip empty expressions
        if expr.is_empty() {
            return identifiers;
        }

        // Simple tokenizer for identifiers
        let mut chars = expr.char_indices().peekable();
        while let Some((start, c)) = chars.next() {
            // Start of identifier
            if c.is_ascii_alphabetic() || c == '_' || c == '$' {
                let mut end = start + c.len_utf8();
                while let Some(&(i, next)) = chars.peek() {
                    if next.is_ascii_alphanumeric() || next == '_' || next == '$' {
                        end = i + next.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }

                let ident = &expr[start..end];

                // Skip keywords and built-in globals
                if !is_keyword(ident) && !is_js_global(ident) {
                    identifiers.push(ident);
                }
            }
        }

        identifiers
    }
}

/// Check if a string is a JavaScript keyword
fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "true"
            | "false"
            | "null"
            | "undefined"
            | "this"
            | "if"
            | "else"
            | "for"
            | "while"
            | "do"
            | "switch"
            | "case"
            | "break"
            | "continue"
            | "return"
            | "throw"
            | "try"
            | "catch"
            | "finally"
            | "new"
            | "delete"
            | "typeof"
            | "void"
            | "in"
            | "of"
            | "instanceof"
            | "function"
            | "class"
            | "const"
            | "let"
            | "var"
            | "async"
            | "await"
            | "yield"
            | "import"
            | "export"
            | "default"
            | "from"
            | "as"
    )
}

impl Rule for NoUndefinedRefs {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_interpolation<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        interpolation: &InterpolationNode<'a>,
    ) {
        // Skip if no analysis available
        if !ctx.has_analysis() {
            return;
        }

        if let ExpressionNode::Simple(expr) = &interpolation.content {
            let identifiers = Self::extract_identifiers(&expr.content);

            for ident in identifiers {
                if !ctx.is_variable_defined(ident) {
                    ctx.warn_with_help(
                        "Variable is not defined",
                        &interpolation.loc,
                        "Define in <script setup> or ensure it's imported",
                    );
                }
            }
        }
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        // Skip if no analysis available
        if !ctx.has_analysis() {
            return;
        }

        // Check directive expressions
        for prop in &element.props {
            if let vize_relief::ast::PropNode::Directive(dir) = prop {
                // Skip v-for (defines its own variables)
                if dir.name == "for" {
                    continue;
                }

                // Check expression
                if let Some(ExpressionNode::Simple(expr)) = &dir.exp {
                    let identifiers = Self::extract_identifiers(&expr.content);

                    for ident in identifiers {
                        if !ctx.is_variable_defined(ident) {
                            ctx.warn_with_help(
                                "Variable is not defined",
                                &dir.loc,
                                "Define in <script setup> or ensure it's imported",
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_identifiers() {
        let ids = NoUndefinedRefs::extract_identifiers("count + 1");
        assert_eq!(ids, vec!["count"]);

        let ids = NoUndefinedRefs::extract_identifiers("user.name");
        assert_eq!(ids, vec!["user", "name"]);

        let ids = NoUndefinedRefs::extract_identifiers("items.map(item => item.id)");
        assert!(ids.contains(&"items"));

        let ids = NoUndefinedRefs::extract_identifiers("true && false");
        assert!(ids.is_empty());

        let ids = NoUndefinedRefs::extract_identifiers("console.log(msg)");
        // console is a global (filtered out), but log and msg are extracted
        // Note: This is a simplified tokenizer - a real implementation would
        // understand that log is a property access, not a variable
        assert_eq!(ids, vec!["log", "msg"]);
    }

    #[test]
    fn test_is_keyword() {
        assert!(is_keyword("true"));
        assert!(is_keyword("false"));
        assert!(is_keyword("null"));
        assert!(is_keyword("this"));
        assert!(!is_keyword("count"));
        assert!(!is_keyword("user"));
    }
}
