//! a11y/no-aria-hidden-on-focusable
//!
//! Disallow `aria-hidden="true"` on focusable elements.
//!
//! Using `aria-hidden="true"` on a focusable element hides it from
//! assistive technologies while it remains focusable by keyboard,
//! creating a confusing experience for screen reader users.
//!
//! Based on eslint-plugin-vuejs-accessibility no-aria-hidden-on-focusable rule.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType};

use super::helpers;

static META: RuleMeta = RuleMeta {
    name: "a11y/no-aria-hidden-on-focusable",
    description: "Disallow aria-hidden=\"true\" on focusable elements",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Error,
};

/// Disallow aria-hidden="true" on focusable elements
#[derive(Default)]
pub struct NoAriaHiddenOnFocusable;

impl Rule for NoAriaHiddenOnFocusable {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        if let Some(value) = helpers::get_static_attribute_value(element, "aria-hidden") {
            if value == "true" && helpers::is_focusable_element(element) {
                ctx.error_with_help(
                    ctx.t("a11y/no-aria-hidden-on-focusable.message"),
                    &element.loc,
                    ctx.t("a11y/no-aria-hidden-on-focusable.help"),
                );
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
        registry.register(Box::new(NoAriaHiddenOnFocusable));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_aria_hidden_on_non_focusable() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div aria-hidden="true"></div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_aria_hidden_on_button() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<button aria-hidden="true">Click</button>"#, "test.vue");
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_valid_aria_hidden_false_on_button() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<button aria-hidden="false">Click</button>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }
}
