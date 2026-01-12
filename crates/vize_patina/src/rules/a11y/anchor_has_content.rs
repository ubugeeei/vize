//! a11y/anchor-has-content
//!
//! Require anchor elements to have accessible content.
//!
//! Anchor elements without content are not accessible to screen reader users.
//! Content can be text, images with alt text, or elements with aria-label.
//!
//! Based on eslint-plugin-vuejs-accessibility anchor-has-content rule.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, PropNode, TemplateChildNode};

static META: RuleMeta = RuleMeta {
    name: "a11y/anchor-has-content",
    description: "Require anchor elements to have accessible content",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Require anchor elements to have accessible content
#[derive(Default)]
pub struct AnchorHasContent;

impl AnchorHasContent {
    fn has_accessible_content(element: &ElementNode) -> bool {
        // Check for aria-label or aria-labelledby
        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                if attr.name == "aria-label" || attr.name == "aria-labelledby" {
                    return true;
                }
            }
            if let PropNode::Directive(dir) = prop {
                if dir.name == "bind" {
                    if let Some(vize_relief::ast::ExpressionNode::Simple(s)) = &dir.arg {
                        if s.content == "aria-label" || s.content == "aria-labelledby" {
                            return true;
                        }
                    }
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
                    // Check for img with alt
                    if el.tag == "img" {
                        for prop in &el.props {
                            if let PropNode::Attribute(attr) = prop {
                                if attr.name == "alt"
                                    && attr.value.as_ref().is_some_and(|v| !v.content.is_empty())
                                {
                                    return true;
                                }
                            }
                        }
                    }
                    // Recursively check other elements
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

impl Rule for AnchorHasContent {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag != "a" {
            return;
        }

        if !Self::has_accessible_content(element) {
            ctx.warn_with_help(
                ctx.t("a11y/anchor-has-content.message"),
                &element.loc,
                ctx.t("a11y/anchor-has-content.help"),
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
        registry.register(Box::new(AnchorHasContent));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_with_text() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<a href="/">Home</a>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_aria_label() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<a href="/" aria-label="Home"></a>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_empty() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<a href="/"></a>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_whitespace_only() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<a href="/">   </a>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
