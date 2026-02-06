//! a11y/label-has-for
//!
//! Require `<label>` elements to have a `for` attribute or wrap a form control.
//!
//! Labels must be properly associated with form controls so assistive
//! technologies can correctly identify and announce them.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <label>Name</label>
//! ```
//!
//! ### Valid
//! ```vue
//! <label for="name">Name</label>
//! <label><input type="text" /> Name</label>
//! <label :for="id">Name</label>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, ExpressionNode, PropNode, TemplateChildNode};

static META: RuleMeta = RuleMeta {
    name: "a11y/label-has-for",
    description: "Require labels to have associated form controls",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Require labels to have associated form controls
#[derive(Default)]
pub struct LabelHasFor;

const FORM_CONTROL_TAGS: &[&str] = &["input", "select", "textarea"];

fn has_for_attribute(element: &ElementNode) -> bool {
    element.props.iter().any(|prop| match prop {
        PropNode::Attribute(attr) => attr.name == "for" || attr.name == "htmlFor",
        PropNode::Directive(dir) => {
            if dir.name == "bind" {
                matches!(
                    &dir.arg,
                    Some(ExpressionNode::Simple(s)) if s.content == "for" || s.content == "htmlFor"
                )
            } else {
                false
            }
        }
    })
}

fn has_nested_form_control(children: &[TemplateChildNode]) -> bool {
    for child in children {
        match child {
            TemplateChildNode::Element(el) => {
                if FORM_CONTROL_TAGS.contains(&el.tag.as_str()) {
                    return true;
                }
                if has_nested_form_control(&el.children) {
                    return true;
                }
            }
            TemplateChildNode::If(if_node) => {
                for branch in &if_node.branches {
                    if has_nested_form_control(&branch.children) {
                        return true;
                    }
                }
            }
            TemplateChildNode::For(for_node) => {
                if has_nested_form_control(&for_node.children) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

impl Rule for LabelHasFor {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        if element.tag != "label" {
            return;
        }

        // Has `for` attribute (static or dynamic)
        if has_for_attribute(element) {
            return;
        }

        // Has nested form control
        if has_nested_form_control(&element.children) {
            return;
        }

        ctx.warn_with_help(
            ctx.t("a11y/label-has-for.message"),
            &element.loc,
            ctx.t("a11y/label-has-for.help"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::Linter;
    use crate::rule::RuleRegistry;

    fn create_linter() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(LabelHasFor));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_with_for() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<label for="name">Name</label>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_dynamic_for() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<label :for="id">Name</label>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_wrapping_input() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<label>Name <input type="text" /></label>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_wrapping_select() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<label>Choice <select><option>A</option></select></label>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_no_for_no_control() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<label>Name</label>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_empty_label() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<label>Name <span>Help</span></label>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
