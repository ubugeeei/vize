//! a11y/iframe-has-title
//!
//! Require iframe elements to have a title attribute.
//!
//! Screen readers use the title attribute to describe the iframe content.
//!
//! Based on eslint-plugin-vuejs-accessibility iframe-has-title rule.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, PropNode};

static META: RuleMeta = RuleMeta {
    name: "a11y/iframe-has-title",
    description: "Require iframe elements to have a title attribute",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Require iframe elements to have a title attribute
#[derive(Default)]
pub struct IframeHasTitle;

impl Rule for IframeHasTitle {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag != "iframe" {
            return;
        }

        // Check for title attribute (static or dynamic)
        let has_title = element.props.iter().any(|prop| match prop {
            PropNode::Attribute(attr) => {
                attr.name == "title"
                    && attr
                        .value
                        .as_ref()
                        .is_some_and(|v| !v.content.trim().is_empty())
            }
            PropNode::Directive(dir) => {
                if dir.name == "bind" {
                    matches!(
                        &dir.arg,
                        Some(vize_relief::ast::ExpressionNode::Simple(s)) if s.content == "title"
                    )
                } else {
                    false
                }
            }
        });

        if !has_title {
            ctx.warn_with_help(
                ctx.t("a11y/iframe-has-title.message"),
                &element.loc,
                ctx.t("a11y/iframe-has-title.help"),
            );
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
        registry.register(Box::new(IframeHasTitle));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_with_title() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<iframe src="https://example.com" title="Example website"></iframe>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_dynamic_title() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<iframe src="https://example.com" :title="frameTitle"></iframe>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_no_title() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<iframe src="https://example.com"></iframe>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_empty_title() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<iframe src="https://example.com" title=""></iframe>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }
}
