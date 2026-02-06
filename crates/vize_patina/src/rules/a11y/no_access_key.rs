//! a11y/no-access-key
//!
//! Disallow the use of the `accesskey` attribute.
//!
//! Access keys are keyboard shortcuts that can conflict with browser
//! and assistive technology shortcuts, creating an inconsistent
//! experience across platforms and devices.
//!
//! Based on eslint-plugin-vuejs-accessibility no-access-key rule.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, PropNode};

static META: RuleMeta = RuleMeta {
    name: "a11y/no-access-key",
    description: "Disallow the use of the accesskey attribute",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Disallow the use of the accesskey attribute
#[derive(Default)]
pub struct NoAccessKey;

impl Rule for NoAccessKey {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                if attr.name == "accesskey" {
                    ctx.warn_with_help(
                        ctx.t("a11y/no-access-key.message"),
                        &attr.loc,
                        ctx.t("a11y/no-access-key.help"),
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
        registry.register(Box::new(NoAccessKey));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_no_accesskey() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>Content</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_has_accesskey() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div accesskey="h">Content</div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
