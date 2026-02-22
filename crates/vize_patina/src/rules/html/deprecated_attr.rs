//! html/deprecated-attr
//!
//! Warn when deprecated HTML attributes are used. Suggests CSS alternatives.
//! Based on markuplint's `deprecated-attr` rule.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <div align="center">text</div>
//!   <table bgcolor="#fff" cellpadding="5">...</table>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template>
//!   <div style="text-align: center">text</div>
//!   <table style="background-color: #fff">...</table>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, PropNode};

use super::helpers::deprecated_attr_suggestion;

static META: RuleMeta = RuleMeta {
    name: "html/deprecated-attr",
    description: "Disallow deprecated HTML attributes",
    category: RuleCategory::HtmlConformance,
    fixable: false,
    default_severity: Severity::Warning,
};

#[derive(Default)]
pub struct DeprecatedAttr;

impl Rule for DeprecatedAttr {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        let tag = element.tag.as_str();

        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                let name = attr.name.as_str();
                if let Some(suggestion) = deprecated_attr_suggestion(tag, name) {
                    let message = ctx.t_fmt(
                        "html/deprecated-attr.message",
                        &[("attr", name), ("tag", tag)],
                    );
                    let help =
                        ctx.t_fmt("html/deprecated-attr.help", &[("suggestion", suggestion)]);
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
        registry.register(Box::new(DeprecatedAttr));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_no_deprecated() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div class="center">text</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_table_border() {
        let linter = create_linter();
        // border on table is NOT deprecated
        let result = linter.lint_template(r#"<table border="1"></table>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_align() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div align="center">text</div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_bgcolor() {
        let linter = create_linter();
        let result = linter.lint_template(r##"<table bgcolor="#fff"></table>"##, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_cellpadding() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<table cellpadding="5"></table>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_td_valign() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<table><tr><td valign="top">text</td></tr></table>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_br_clear() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<br clear="all">"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_multiple() {
        let linter = create_linter();
        let result = linter.lint_template(
            r##"<div align="center" bgcolor="#fff">text</div>"##,
            "test.vue",
        );
        assert_eq!(result.warning_count, 2);
    }
}
