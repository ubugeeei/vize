//! vue/no-v-text-v-html-on-component
//!
//! Disallow v-text / v-html on component elements.
//!
//! Using `v-text` or `v-html` on components is an error because components
//! have their own rendering logic and these directives would overwrite
//! the component's content in an unexpected way.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <MyComponent v-html="content" />
//! <MyComponent v-text="content" />
//! ```
//!
//! ### Valid
//! ```vue
//! <div v-html="content"></div>
//! <MyComponent>{{ content }}</MyComponent>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{DirectiveNode, ElementNode, ElementType};

static META: RuleMeta = RuleMeta {
    name: "vue/no-v-text-v-html-on-component",
    description: "Disallow v-text / v-html on component elements",
    category: RuleCategory::Essential,
    fixable: false,
    default_severity: Severity::Error,
};

#[derive(Default)]
pub struct NoVTextVHtmlOnComponent;

impl Rule for NoVTextVHtmlOnComponent {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_directive<'a>(
        &self,
        ctx: &mut LintContext<'a>,
        element: &ElementNode<'a>,
        directive: &DirectiveNode<'a>,
    ) {
        if directive.name != "text" && directive.name != "html" {
            return;
        }

        if element.tag_type != ElementType::Component {
            return;
        }

        ctx.error_with_help(
            ctx.t_fmt(
                "vue/no-v-text-v-html-on-component.message",
                &[
                    ("directive", directive.name.as_str()),
                    ("tag", element.tag.as_str()),
                ],
            ),
            &directive.loc,
            ctx.t("vue/no-v-text-v-html-on-component.help"),
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
        registry.register(Box::new(NoVTextVHtmlOnComponent));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_v_html_on_div() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div v-html="content"></div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_v_text_on_span() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<span v-text="content"></span>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_v_html_on_component() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<MyComponent v-html="content" />"#, "test.vue");
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_invalid_v_text_on_component() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<MyComponent v-text="content" />"#, "test.vue");
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_valid_component_with_slot_content() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<MyComponent>{{ content }}</MyComponent>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }
}
