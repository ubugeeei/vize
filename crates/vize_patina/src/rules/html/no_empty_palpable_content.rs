//! html/no-empty-palpable-content
//!
//! Detect elements that should have visible (palpable) content but are empty.
//! Based on markuplint's `no-empty-palpable-content` rule.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <p></p>
//!   <li></li>
//!   <td></td>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template>
//!   <p>text</p>
//!   <li>item</li>
//!   <td>cell</td>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, PropNode};

use super::helpers::{has_palpable_content, PALPABLE_CONTENT_ELEMENTS};

static META: RuleMeta = RuleMeta {
    name: "html/no-empty-palpable-content",
    description: "Disallow empty elements that expect visible content",
    category: RuleCategory::HtmlConformance,
    fixable: false,
    default_severity: Severity::Warning,
};

#[derive(Default)]
pub struct NoEmptyPalpableContent;

impl Rule for NoEmptyPalpableContent {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        let tag = element.tag.as_str();
        if !PALPABLE_CONTENT_ELEMENTS.contains(&tag) {
            return;
        }

        // Skip if element has aria-label, v-html, v-text, or slot
        let has_content_source = element.props.iter().any(|prop| match prop {
            PropNode::Attribute(attr) => {
                attr.name == "aria-label" || attr.name == "aria-labelledby"
            }
            PropNode::Directive(dir) => {
                dir.name == "html" || dir.name == "text" || dir.name == "slot"
            }
        });

        if has_content_source {
            return;
        }

        if !has_palpable_content(element) {
            let message = ctx.t_fmt("html/no-empty-palpable-content.message", &[("tag", tag)]);
            let help = ctx.t("html/no-empty-palpable-content.help");
            ctx.warn_with_help(message, &element.loc, help);
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
        registry.register(Box::new(NoEmptyPalpableContent));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_with_text() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<p>text</p>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_interpolation() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<p>{{ text }}</p>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_child_element() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<p><span>text</span></p>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_aria_label() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<p aria-label="description"></p>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_v_html() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<p v-html="content"></p>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_v_text() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<p v-text="content"></p>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_div_empty() {
        let linter = create_linter();
        // div is NOT in palpable content elements
        let result = linter.lint_template(r#"<div></div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_empty_p() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<p></p>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_whitespace_only_p() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<p>   </p>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_empty_li() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<ul><li></li></ul>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_empty_td() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<table><tr><td></td></tr></table>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_empty_option() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<select><option></option></select>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
