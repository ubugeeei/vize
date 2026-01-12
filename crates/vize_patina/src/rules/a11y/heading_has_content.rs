//! a11y/heading-has-content
//!
//! Require heading elements (h1-h6) to have accessible content.
//!
//! Empty headings are not accessible to screen reader users.
//!
//! Based on eslint-plugin-vuejs-accessibility heading-has-content rule.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, PropNode, TemplateChildNode};

static META: RuleMeta = RuleMeta {
    name: "a11y/heading-has-content",
    description: "Require heading elements to have accessible content",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Require heading elements to have accessible content
#[derive(Default)]
pub struct HeadingHasContent;

impl HeadingHasContent {
    fn is_heading(tag: &str) -> bool {
        matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
    }

    fn has_accessible_content(element: &ElementNode) -> bool {
        // Check for aria-label
        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                if attr.name == "aria-label" {
                    return true;
                }
            }
        }

        // Check for content in children
        for child in &element.children {
            match child {
                TemplateChildNode::Text(text) => {
                    if !text.content.trim().is_empty() {
                        return true;
                    }
                }
                TemplateChildNode::Interpolation(_) => {
                    return true;
                }
                TemplateChildNode::Element(el) => {
                    if Self::has_accessible_content(el) {
                        return true;
                    }
                }
                _ => {}
            }
        }

        false
    }
}

impl Rule for HeadingHasContent {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if !Self::is_heading(&element.tag) {
            return;
        }

        // Check for aria-hidden="true" (skip check if hidden)
        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                if attr.name == "aria-hidden" {
                    if let Some(val) = &attr.value {
                        if val.content == "true" {
                            return;
                        }
                    }
                }
            }
        }

        if !Self::has_accessible_content(element) {
            ctx.warn_with_help(
                ctx.t_fmt(
                    "a11y/heading-has-content.message",
                    &[("tag", element.tag.as_str())],
                ),
                &element.loc,
                ctx.t("a11y/heading-has-content.help"),
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
        registry.register(Box::new(HeadingHasContent));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_with_text() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<h1>Hello World</h1>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_interpolation() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<h2>{{ title }}</h2>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_aria_hidden() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<h1 aria-hidden="true"></h1>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_empty() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<h1></h1>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
