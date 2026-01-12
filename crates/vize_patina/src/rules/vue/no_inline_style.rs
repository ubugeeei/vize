//! vue/no-inline-style
//!
//! Discourage use of inline style attributes.
//!
//! Inline styles make it harder to maintain consistent styling,
//! can override CSS classes unexpectedly, and reduce code reusability.
//! Prefer using CSS classes, scoped styles, or CSS-in-JS solutions.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <div style="color: red">text</div>
//! <span :style="{ color: 'red' }">text</span>
//! <p :style="dynamicStyles">text</p>
//! ```
//!
//! ### Valid
//! ```vue
//! <div class="text-red">text</div>
//! <span :class="{ 'text-red': isRed }">text</span>
//! ```
//!
//! ### Exceptions
//! Dynamic styles for animations, canvas-like positioning, or user-customizable
//! theming may be acceptable exceptions. This rule can be disabled with a comment.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ExpressionNode};

static META: RuleMeta = RuleMeta {
    name: "vue/no-inline-style",
    description: "Discourage use of inline style attributes",
    category: RuleCategory::Recommended,
    fixable: false,
    default_severity: Severity::Warning,
};

/// No inline style rule
#[derive(Default)]
pub struct NoInlineStyle;

impl Rule for NoInlineStyle {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        // Check for static style attribute
        for attr in &element.props {
            if let vize_relief::ast::PropNode::Attribute(attr) = attr {
                if attr.name == "style" {
                    ctx.warn_with_help(
                        "Avoid using inline style attribute",
                        &attr.loc,
                        "Use CSS classes or scoped styles instead: <div class=\"my-class\">",
                    );
                }
            }

            // Check for dynamic :style binding
            if let vize_relief::ast::PropNode::Directive(dir) = attr {
                if dir.name == "bind" {
                    if let Some(arg) = &dir.arg {
                        let arg_content = match arg {
                            ExpressionNode::Simple(s) => s.content.as_str(),
                            _ => "",
                        };
                        if arg_content == "style" {
                            ctx.warn_with_help(
                                "Avoid using dynamic :style binding",
                                &dir.loc,
                                "Use :class binding for conditional styling: :class=\"{ 'class-name': condition }\"",
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
    use crate::linter::Linter;
    use crate::rule::RuleRegistry;

    fn create_linter() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(NoInlineStyle));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_class() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div class="foo">text</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_dynamic_class() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div :class="{ active: isActive }">text</div>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_static_style() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div style="color: red">text</div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_dynamic_style() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<div :style="{ color: 'red' }">text</div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
