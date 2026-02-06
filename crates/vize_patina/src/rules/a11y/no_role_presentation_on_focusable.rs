//! a11y/no-role-presentation-on-focusable
//!
//! Disallow `role="presentation"` or `role="none"` on focusable elements.
//!
//! Using `role="presentation"` or `role="none"` on a focusable element
//! removes its semantic meaning from the accessibility tree while it
//! remains focusable, creating a confusing experience for screen reader users.
//!
//! Based on eslint-plugin-vuejs-accessibility no-role-presentation-on-focusable rule.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType};

use super::helpers;

static META: RuleMeta = RuleMeta {
    name: "a11y/no-role-presentation-on-focusable",
    description: "Disallow role=\"presentation\" or role=\"none\" on focusable elements",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Error,
};

/// Disallow role="presentation" or role="none" on focusable elements
#[derive(Default)]
pub struct NoRolePresentationOnFocusable;

impl Rule for NoRolePresentationOnFocusable {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        if let Some(role) = helpers::get_static_attribute_value(element, "role") {
            if (role == "presentation" || role == "none") && helpers::is_focusable_element(element)
            {
                ctx.error_with_help(
                    ctx.t("a11y/no-role-presentation-on-focusable.message"),
                    &element.loc,
                    ctx.t("a11y/no-role-presentation-on-focusable.help"),
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
        registry.register(Box::new(NoRolePresentationOnFocusable));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_role_presentation_on_non_focusable() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div role="presentation"></div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_role_presentation_on_button() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<button role="presentation">Click</button>"#, "test.vue");
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_invalid_role_none_on_input() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input role="none" type="text" />"#, "test.vue");
        assert_eq!(result.error_count, 1);
    }
}
