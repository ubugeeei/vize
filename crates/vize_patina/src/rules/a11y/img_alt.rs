//! a11y/img-alt
//!
//! Require alt attribute on <img> elements for accessibility.
//!
//! Images must have an alt attribute for screen readers.
//! Decorative images should have an empty alt attribute.
//!
//! Based on eslint-plugin-vuejs-accessibility alt-text rule.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::ElementNode;

static META: RuleMeta = RuleMeta {
    name: "a11y/img-alt",
    description: "Require alt attribute on images for accessibility",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Require alt attribute on images
#[derive(Default)]
pub struct ImgAlt;

impl Rule for ImgAlt {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag != "img" {
            return;
        }

        // Check for alt attribute (static or dynamic)
        let has_alt = element.props.iter().any(|prop| match prop {
            vize_relief::ast::PropNode::Attribute(attr) => attr.name == "alt",
            vize_relief::ast::PropNode::Directive(dir) => {
                if dir.name == "bind" {
                    matches!(
                        &dir.arg,
                        Some(vize_relief::ast::ExpressionNode::Simple(s)) if s.content == "alt"
                    )
                } else {
                    false
                }
            }
        });

        if !has_alt {
            ctx.warn_with_help(
                ctx.t("a11y/img-alt.message"),
                &element.loc,
                ctx.t("a11y/img-alt.help"),
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
        registry.register(Box::new(ImgAlt));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_with_alt() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<img src="/photo.jpg" alt="Photo" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_empty_alt() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<img src="/decoration.svg" alt="" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_no_alt() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<img src="/photo.jpg" />"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
