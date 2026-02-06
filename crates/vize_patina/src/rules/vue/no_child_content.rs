//! vue/no-child-content
//!
//! Disallow element's child contents which would be overwritten by a directive
//! like `v-html` or `v-text`.
//!
//! When using `v-html` or `v-text`, the child content of the element is ignored
//! by Vue and replaced with the directive's value. Having child content alongside
//! these directives is misleading.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <div v-html="content">child</div>
//! <div v-text="content">child</div>
//! ```
//!
//! ### Valid
//! ```vue
//! <div v-html="content"></div>
//! <div v-text="content"></div>
//! <div>child</div>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, PropNode, TemplateChildNode};

static META: RuleMeta = RuleMeta {
    name: "vue/no-child-content",
    description: "Disallow child content when using v-html or v-text",
    category: RuleCategory::Essential,
    fixable: false,
    default_severity: Severity::Error,
};

#[derive(Default)]
pub struct NoChildContent;

impl NoChildContent {
    /// Returns true if the element has v-html or v-text, and the directive name ("html" or "text")
    fn has_v_html_or_v_text(element: &ElementNode) -> Option<&'static str> {
        for prop in &element.props {
            if let PropNode::Directive(dir) = prop {
                if dir.name == "html" {
                    return Some("html");
                }
                if dir.name == "text" {
                    return Some("text");
                }
            }
        }
        None
    }

    fn has_child_content(element: &ElementNode) -> bool {
        for child in &element.children {
            match child {
                TemplateChildNode::Text(text) => {
                    if !text.content.trim().is_empty() {
                        return true;
                    }
                }
                TemplateChildNode::Element(_)
                | TemplateChildNode::Interpolation(_)
                | TemplateChildNode::If(_)
                | TemplateChildNode::For(_) => {
                    return true;
                }
                _ => {}
            }
        }
        false
    }
}

impl Rule for NoChildContent {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if let Some(directive_name) = Self::has_v_html_or_v_text(element) {
            if Self::has_child_content(element) {
                ctx.error_with_help(
                    ctx.t_fmt(
                        "vue/no-child-content.message",
                        &[("directive", directive_name)],
                    ),
                    &element.loc,
                    ctx.t("vue/no-child-content.help"),
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
        registry.register(Box::new(NoChildContent));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_v_html_no_children() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div v-html="content"></div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_v_text_no_children() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div v-text="content"></div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_no_directive() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>child content</div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_v_html_with_text() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div v-html="content">child</div>"#, "test.vue");
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_invalid_v_text_with_text() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div v-text="content">child</div>"#, "test.vue");
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_invalid_v_html_with_element() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div v-html="content"><span>child</span></div>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_valid_v_html_whitespace_only() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div v-html="content">   </div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }
}
