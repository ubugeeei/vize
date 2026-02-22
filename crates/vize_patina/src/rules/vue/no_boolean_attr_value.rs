//! vue/no-boolean-attr-value
//!
//! Warn when boolean HTML attributes have explicit values.
//! For example, `disabled="disabled"` should be just `disabled`.
//! Based on markuplint's `no-boolean-attr-value` rule.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <input disabled="disabled" />
//!   <input checked="checked" />
//!   <button disabled="true">Click</button>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template>
//!   <input disabled />
//!   <input checked />
//!   <button disabled>Click</button>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use crate::rules::html::helpers::BOOLEAN_ATTRIBUTES;
use vize_relief::ast::{ElementNode, ElementType, PropNode};

static META: RuleMeta = RuleMeta {
    name: "vue/no-boolean-attr-value",
    description: "Disallow explicit values for boolean HTML attributes",
    category: RuleCategory::Recommended,
    fixable: true,
    default_severity: Severity::Warning,
};

#[derive(Default)]
pub struct NoBooleanAttrValue;

impl Rule for NoBooleanAttrValue {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                let name = attr.name.as_str();
                if !BOOLEAN_ATTRIBUTES.contains(&name) {
                    continue;
                }

                if let Some(value) = &attr.value {
                    // Has an explicit value — warn
                    let message = ctx.t_fmt(
                        "vue/no-boolean-attr-value.message",
                        &[("attr", name), ("value", value.content.as_str())],
                    );
                    let help = ctx.t_fmt("vue/no-boolean-attr-value.help", &[("attr", name)]);
                    ctx.warn_with_help(message, &attr.loc, help);
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
        registry.register(Box::new(NoBooleanAttrValue));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_no_value() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input disabled />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_checked_no_value() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input checked />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_non_boolean_with_value() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input type="text" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_dynamic_binding() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input :disabled="isDisabled" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_disabled_with_value() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input disabled="disabled" />"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_checked_with_value() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input checked="checked" />"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_disabled_true() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<button disabled="true">Click</button>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_multiple() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<input disabled="disabled" required="required" />"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 2);
    }

    #[test]
    fn test_invalid_hidden_with_value() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div hidden="hidden">text</div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
