//! html/require-datetime
//!
//! Require `datetime` attribute on `<time>` element when its text content
//! is not a valid datetime string.
//! Based on markuplint's `require-datetime` rule.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <time>last Tuesday</time>
//!   <time>Christmas</time>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template>
//!   <time datetime="2024-12-25">Christmas</time>
//!   <time>2024-12-25</time>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, PropNode, TemplateChildNode};

use super::helpers::is_valid_datetime;

static META: RuleMeta = RuleMeta {
    name: "html/require-datetime",
    description: "Require datetime attribute on <time> element",
    category: RuleCategory::HtmlConformance,
    fixable: false,
    default_severity: Severity::Warning,
};

#[derive(Default)]
pub struct RequireDatetime;

impl Rule for RequireDatetime {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag != "time" {
            return;
        }

        // Check if datetime attribute exists (static or dynamic)
        let has_datetime = element.props.iter().any(|prop| match prop {
            PropNode::Attribute(attr) => attr.name == "datetime",
            PropNode::Directive(dir) => {
                dir.name == "bind"
                    && dir.arg.as_ref().is_some_and(|arg| {
                        if let vize_relief::ast::ExpressionNode::Simple(s) = arg {
                            s.content == "datetime"
                        } else {
                            false
                        }
                    })
            }
        });

        if has_datetime {
            return;
        }

        // Check if text content is a valid datetime
        let mut text_content = String::new();
        let mut has_dynamic_content = false;

        for child in &element.children {
            match child {
                TemplateChildNode::Text(text) => {
                    text_content.push_str(text.content.as_str());
                }
                TemplateChildNode::Interpolation(_) => {
                    has_dynamic_content = true;
                }
                _ => {}
            }
        }

        // If there's dynamic content, we can't validate
        if has_dynamic_content {
            return;
        }

        if !is_valid_datetime(&text_content) {
            let message = ctx.t("html/require-datetime.message");
            let help = ctx.t("html/require-datetime.help");
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
        registry.register(Box::new(RequireDatetime));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_with_datetime_attr() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<time datetime="2024-12-25">Christmas</time>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_dynamic_datetime() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<time :datetime="date">some text</time>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_iso_date_content() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<time>2024-12-25</time>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_iso_datetime_content() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<time>2024-12-25T10:30</time>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_dynamic_interpolation() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<time>{{ formattedDate }}</time>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_text_content() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<time>last Tuesday</time>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_human_readable() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<time>Christmas Day</time>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_duration() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<time>PT1H30M</time>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }
}
