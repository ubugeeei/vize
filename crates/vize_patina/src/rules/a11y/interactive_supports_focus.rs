//! a11y/interactive-supports-focus
//!
//! Require elements with interactive roles to be focusable.
//!
//! Elements that have interactive ARIA roles (button, link, etc.) must
//! be focusable either natively or via tabindex.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <div role="button" @click="handle">Click</div>
//! ```
//!
//! ### Valid
//! ```vue
//! <div role="button" tabindex="0" @click="handle">Click</div>
//! <button @click="handle">Click</button>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType};

use super::helpers::{
    get_static_attribute_value, is_focusable_element, is_interactive_element, is_interactive_role,
};

static META: RuleMeta = RuleMeta {
    name: "a11y/interactive-supports-focus",
    description: "Require interactive role elements to be focusable",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Require interactive role elements to be focusable
#[derive(Default)]
pub struct InteractiveSupportsFocus;

impl Rule for InteractiveSupportsFocus {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        // Skip natively interactive elements - they're already focusable
        if is_interactive_element(&element.tag) {
            return;
        }

        // Check if element has an interactive role
        let role = match get_static_attribute_value(element, "role") {
            Some(r) => r,
            None => return,
        };

        if !is_interactive_role(role) {
            return;
        }

        // Element has interactive role but is not natively interactive
        // Check if it's focusable
        if !is_focusable_element(element) {
            ctx.warn_with_help(
                ctx.t_fmt("a11y/interactive-supports-focus.message", &[("role", role)]),
                &element.loc,
                ctx.t("a11y/interactive-supports-focus.help"),
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
        registry.register(Box::new(InteractiveSupportsFocus));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_native_button() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<button @click="handle">Click</button>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_div_role_button_with_tabindex() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div role="button" tabindex="0" @click="handle">Click</div>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_div_role_button_no_tabindex() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div role="button" @click="handle">Click</div>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_non_interactive_role() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div role="presentation">Content</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_span_role_link() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<span role="link">Link</span>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_component_skipped() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<MyButton role="button">Click</MyButton>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }
}
