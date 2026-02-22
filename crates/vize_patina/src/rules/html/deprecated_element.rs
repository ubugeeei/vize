//! html/deprecated-element
//!
//! Warn when deprecated or obsolete HTML elements are used.
//! Based on markuplint's `deprecated-element` rule.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <center>centered text</center>
//!   <font color="red">colored text</font>
//!   <marquee>scrolling text</marquee>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template>
//!   <div style="text-align: center">centered text</div>
//!   <span style="color: red">colored text</span>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType};

use super::helpers::DEPRECATED_ELEMENTS;

static META: RuleMeta = RuleMeta {
    name: "html/deprecated-element",
    description: "Disallow deprecated HTML elements",
    category: RuleCategory::HtmlConformance,
    fixable: false,
    default_severity: Severity::Warning,
};

#[derive(Default)]
pub struct DeprecatedElement;

impl Rule for DeprecatedElement {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        let tag = element.tag.as_str();
        if DEPRECATED_ELEMENTS.contains(&tag) {
            let message = ctx.t_fmt("html/deprecated-element.message", &[("tag", tag)]);
            let help = ctx.t("html/deprecated-element.help");
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
        registry.register(Box::new(DeprecatedElement));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_modern_elements() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div><p>text</p></div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_component() {
        let linter = create_linter();
        // Component names that happen to match deprecated elements should be skipped
        let result = linter.lint_template(r#"<MyCenter>text</MyCenter>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_center() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<center>text</center>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_font() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<font color="red">text</font>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_marquee() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<marquee>scrolling</marquee>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_big() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<big>large text</big>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_tt() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<tt>monospace</tt>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_strike() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<strike>deleted</strike>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_multiple() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div><center>text</center><font>text</font></div>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 2);
    }
}
