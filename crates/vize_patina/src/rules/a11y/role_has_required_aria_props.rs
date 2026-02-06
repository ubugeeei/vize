//! a11y/role-has-required-aria-props
//!
//! Require elements with ARIA roles to have all required ARIA properties.
//!
//! Some ARIA roles require specific ARIA attributes to be present.
//! For example, `role="checkbox"` requires `aria-checked`.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <div role="checkbox">Check</div>
//! <div role="slider">Slider</div>
//! ```
//!
//! ### Valid
//! ```vue
//! <div role="checkbox" aria-checked="false">Check</div>
//! <div role="slider" aria-valuenow="50" aria-valuemin="0" aria-valuemax="100">Slider</div>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, ExpressionNode, PropNode};

use super::helpers::{get_required_aria_props, get_static_attribute_value};

static META: RuleMeta = RuleMeta {
    name: "a11y/role-has-required-aria-props",
    description: "Require ARIA roles to have required properties",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Require ARIA roles to have required properties
#[derive(Default)]
pub struct RoleHasRequiredAriaProps;

fn has_aria_attribute_or_binding(element: &ElementNode, name: &str) -> bool {
    element.props.iter().any(|prop| match prop {
        PropNode::Attribute(attr) => attr.name == name,
        PropNode::Directive(dir) => {
            if dir.name == "bind" {
                matches!(
                    &dir.arg,
                    Some(ExpressionNode::Simple(s)) if s.content == name
                )
            } else {
                false
            }
        }
    })
}

impl Rule for RoleHasRequiredAriaProps {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        let role = match get_static_attribute_value(element, "role") {
            Some(r) => r,
            None => return,
        };

        let required_props = get_required_aria_props(role);
        if required_props.is_empty() {
            return;
        }

        for &required_prop in required_props {
            if !has_aria_attribute_or_binding(element, required_prop) {
                ctx.warn_with_help(
                    ctx.t_fmt(
                        "a11y/role-has-required-aria-props.message",
                        &[("role", role), ("prop", required_prop)],
                    ),
                    &element.loc,
                    ctx.t_fmt(
                        "a11y/role-has-required-aria-props.help",
                        &[("role", role), ("props", &required_props.join(", "))],
                    ),
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
        registry.register(Box::new(RoleHasRequiredAriaProps));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_checkbox_with_checked() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div role="checkbox" aria-checked="false">Check</div>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_checkbox_with_dynamic_checked() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div role="checkbox" :aria-checked="checked">Check</div>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_checkbox_missing_checked() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div role="checkbox">Check</div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_slider_missing_valuenow() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div role="slider">Slider</div>"#, "test.vue");
        // slider requires aria-valuenow
        assert!(result.warning_count >= 1);
    }

    #[test]
    fn test_valid_no_required_props() {
        let linter = create_linter();
        // "button" role has no required ARIA props
        let result = linter.lint_template(r#"<div role="button">Button</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_no_role() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>Content</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_component_skipped() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<MyCheckbox role="checkbox">Check</MyCheckbox>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }
}
