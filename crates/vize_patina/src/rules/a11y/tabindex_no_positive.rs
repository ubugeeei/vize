//! a11y/tabindex-no-positive
//!
//! Disallow positive tabindex values.
//!
//! Positive tabindex values disrupt the natural tab order and can make
//! navigation confusing for keyboard users. Use 0 or -1 instead.
//!
//! Based on eslint-plugin-vuejs-accessibility tabindex-no-positive rule.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, PropNode};

static META: RuleMeta = RuleMeta {
    name: "a11y/tabindex-no-positive",
    description: "Disallow positive tabindex values",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Disallow positive tabindex values
#[derive(Default)]
pub struct TabindexNoPositive;

impl Rule for TabindexNoPositive {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                if attr.name == "tabindex" {
                    if let Some(value) = &attr.value {
                        if let Ok(num) = value.content.parse::<i32>() {
                            if num > 0 {
                                ctx.warn_with_help(
                                    ctx.t("a11y/tabindex-no-positive.message"),
                                    &attr.loc,
                                    ctx.t("a11y/tabindex-no-positive.help"),
                                );
                            }
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
        registry.register(Box::new(TabindexNoPositive));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_zero() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div tabindex="0">Focusable</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_negative() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<div tabindex="-1">Programmatic focus</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_positive() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div tabindex="1">Bad focus order</div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_large_positive() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div tabindex="99">Very bad</div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
