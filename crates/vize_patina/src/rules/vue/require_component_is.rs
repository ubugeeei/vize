//! vue/require-component-is
//!
//! Require `v-bind:is` attribute on `<component>` elements.
//!
//! The `<component>` element is a dynamic component that requires an `is` or
//! `:is` attribute to determine which component to render.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <component />
//! <component></component>
//! ```
//!
//! ### Valid
//! ```vue
//! <component :is="currentComponent" />
//! <component is="MyComponent" />
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ExpressionNode, PropNode};

static META: RuleMeta = RuleMeta {
    name: "vue/require-component-is",
    description: "Require `v-bind:is` on `<component>` elements",
    category: RuleCategory::Essential,
    fixable: false,
    default_severity: Severity::Error,
};

#[derive(Default)]
pub struct RequireComponentIs;

impl RequireComponentIs {
    fn has_is_attribute(element: &ElementNode) -> bool {
        for prop in &element.props {
            match prop {
                PropNode::Attribute(attr) => {
                    if attr.name == "is" {
                        return true;
                    }
                }
                PropNode::Directive(dir) => {
                    // Check for :is or v-bind:is
                    if dir.name == "bind" {
                        if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                            if arg.content == "is" {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }
}

impl Rule for RequireComponentIs {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag.as_str() != "component" {
            return;
        }

        if !Self::has_is_attribute(element) {
            ctx.error_with_help(
                ctx.t("vue/require-component-is.message"),
                &element.loc,
                ctx.t("vue/require-component-is.help"),
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
        registry.register(Box::new(RequireComponentIs));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_with_bind_is() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<component :is="currentComponent" />"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_with_static_is() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<component is="MyComponent" />"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_no_is() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<component />"#, "test.vue");
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_invalid_no_is_with_children() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<component>content</component>"#, "test.vue");
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_not_component_tag() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div></div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }
}
