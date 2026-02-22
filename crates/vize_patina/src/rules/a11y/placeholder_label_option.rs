//! a11y/placeholder-label-option
//!
//! Validate `<select>` placeholder label requirements.
//! The first `<option>` with an empty `value` attribute should have
//! `disabled` or `hidden` to be a proper placeholder.
//! Based on markuplint's `placeholder-label-option` rule.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <select>
//!     <option value="">Choose one</option>
//!     <option value="a">A</option>
//!   </select>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template>
//!   <select>
//!     <option value="" disabled>Choose one</option>
//!     <option value="a">A</option>
//!   </select>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, PropNode, TemplateChildNode};

static META: RuleMeta = RuleMeta {
    name: "a11y/placeholder-label-option",
    description: "Require disabled or hidden on select placeholder option",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

#[derive(Default)]
pub struct PlaceholderLabelOption;

impl Rule for PlaceholderLabelOption {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component || element.tag != "select" {
            return;
        }

        // Find first <option> child
        let first_option = element.children.iter().find_map(|child| {
            if let TemplateChildNode::Element(el) = child {
                if el.tag == "option" {
                    return Some(el.as_ref());
                }
            }
            None
        });

        let Some(option) = first_option else {
            return;
        };

        // Check if it's a placeholder (value="" or no value attribute)
        let is_placeholder = option.props.iter().any(|prop| {
            if let PropNode::Attribute(attr) = prop {
                if attr.name == "value" {
                    return attr.value.as_ref().map_or(true, |v| v.content.is_empty());
                }
            }
            false
        });

        if !is_placeholder {
            return;
        }

        // Check if disabled or hidden
        let has_disabled_or_hidden = option.props.iter().any(|prop| {
            if let PropNode::Attribute(attr) = prop {
                attr.name == "disabled" || attr.name == "hidden"
            } else {
                false
            }
        });

        if !has_disabled_or_hidden {
            let message = ctx.t("a11y/placeholder-label-option.message");
            let help = ctx.t("a11y/placeholder-label-option.help");
            ctx.warn_with_help(message, &option.loc, help);
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
        registry.register(Box::new(PlaceholderLabelOption));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_disabled_placeholder() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<select><option value="" disabled>Choose</option><option value="a">A</option></select>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_hidden_placeholder() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<select><option value="" hidden>Choose</option><option value="a">A</option></select>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_no_placeholder() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<select><option value="a">A</option><option value="b">B</option></select>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_no_options() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<select></select>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_not_select() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>text</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_no_disabled_or_hidden() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<select><option value="">Choose</option><option value="a">A</option></select>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }
}
