//! a11y/form-control-has-label
//!
//! Require form controls to have associated labels.
//!
//! Form controls (input, select, textarea) must have associated labels
//! for screen reader users. This can be via <label>, aria-label, or
//! aria-labelledby.
//!
//! Based on eslint-plugin-vuejs-accessibility form-control-has-label rule.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ExpressionNode, PropNode};

static META: RuleMeta = RuleMeta {
    name: "a11y/form-control-has-label",
    description: "Require form controls to have associated labels",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Require form controls to have associated labels
#[derive(Default)]
pub struct FormControlHasLabel;

impl FormControlHasLabel {
    /// Check if an element is a form control that needs a label
    fn is_form_control(tag: &str) -> bool {
        matches!(tag, "input" | "select" | "textarea")
    }

    /// Check if the input type doesn't need a label (hidden, submit, etc.)
    fn is_exempt_input_type(element: &ElementNode) -> bool {
        if element.tag != "input" {
            return false;
        }

        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                if attr.name == "type" {
                    if let Some(value) = &attr.value {
                        return matches!(
                            value.content.as_ref(),
                            "hidden" | "submit" | "reset" | "button" | "image"
                        );
                    }
                }
            }
        }
        false
    }

    /// Check if element has aria-label or aria-labelledby
    fn has_aria_label(element: &ElementNode) -> bool {
        for prop in &element.props {
            match prop {
                PropNode::Attribute(attr) => {
                    if (attr.name == "aria-label" || attr.name == "aria-labelledby")
                        && attr
                            .value
                            .as_ref()
                            .is_some_and(|v| !v.content.trim().is_empty())
                    {
                        return true;
                    }
                }
                PropNode::Directive(dir) => {
                    if dir.name == "bind" {
                        if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                            if arg.content == "aria-label" || arg.content == "aria-labelledby" {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Check if element has an id (potentially used by a label)
    fn has_id(element: &ElementNode) -> bool {
        for prop in &element.props {
            match prop {
                PropNode::Attribute(attr) => {
                    if attr.name == "id"
                        && attr
                            .value
                            .as_ref()
                            .is_some_and(|v| !v.content.trim().is_empty())
                    {
                        return true;
                    }
                }
                PropNode::Directive(dir) => {
                    if dir.name == "bind" {
                        if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                            if arg.content == "id" {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Check if element has a placeholder (weak but sometimes acceptable)
    fn has_placeholder(element: &ElementNode) -> bool {
        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                if attr.name == "placeholder"
                    && attr
                        .value
                        .as_ref()
                        .is_some_and(|v| !v.content.trim().is_empty())
                {
                    return true;
                }
            }
        }
        false
    }

    /// Check if element has a title attribute
    fn has_title(element: &ElementNode) -> bool {
        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                if attr.name == "title"
                    && attr
                        .value
                        .as_ref()
                        .is_some_and(|v| !v.content.trim().is_empty())
                {
                    return true;
                }
            }
        }
        false
    }
}

impl Rule for FormControlHasLabel {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if !Self::is_form_control(&element.tag) {
            return;
        }

        // Skip inputs that don't need labels
        if Self::is_exempt_input_type(element) {
            return;
        }

        // Check for various label methods
        let has_label =
            Self::has_aria_label(element) || Self::has_id(element) || Self::has_title(element);

        if !has_label {
            let help = if Self::has_placeholder(element) {
                ctx.t("a11y/form-control-has-label.help_placeholder")
            } else {
                ctx.t("a11y/form-control-has-label.help")
            };

            ctx.warn_with_help(
                ctx.t_fmt(
                    "a11y/form-control-has-label.message",
                    &[("tag", element.tag.as_str())],
                ),
                &element.loc,
                help,
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
        registry.register(Box::new(FormControlHasLabel));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_with_id() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input type="text" id="name" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_aria_label() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input type="text" aria-label="Name" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_hidden_input() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input type="hidden" value="token" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_submit_button() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input type="submit" value="Submit" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_no_label() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input type="text" />"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_select_no_label() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<select><option>A</option></select>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_textarea_no_label() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<textarea></textarea>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
