//! a11y/no-autofocus
//!
//! Disallow the use of the `autofocus` attribute.
//!
//! The autofocus attribute can cause usability issues for sighted and
//! non-sighted users by moving focus unexpectedly, disrupting the
//! natural reading order and navigation flow.
//!
//! Based on eslint-plugin-vuejs-accessibility no-autofocus rule.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, PropNode};

static META: RuleMeta = RuleMeta {
    name: "a11y/no-autofocus",
    description: "Disallow the use of the autofocus attribute",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Disallow the use of the autofocus attribute
#[derive(Default)]
pub struct NoAutofocus;

impl Rule for NoAutofocus {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                if attr.name == "autofocus" {
                    ctx.warn_with_help(
                        ctx.t("a11y/no-autofocus.message"),
                        &attr.loc,
                        ctx.t("a11y/no-autofocus.help"),
                    );
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
        registry.register(Box::new(NoAutofocus));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_no_autofocus() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input type="text" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_has_autofocus() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input type="text" autofocus />"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
