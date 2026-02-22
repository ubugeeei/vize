//! html/no-duplicate-dt
//!
//! Detect duplicate `<dt>` term definitions within a `<dl>` element.
//! Based on markuplint's `no-duplicate-dt` rule.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <dl>
//!     <dt>Term A</dt>
//!     <dd>Definition 1</dd>
//!     <dt>Term A</dt>
//!     <dd>Definition 2</dd>
//!   </dl>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template>
//!   <dl>
//!     <dt>Term A</dt>
//!     <dd>Definition A</dd>
//!     <dt>Term B</dt>
//!     <dd>Definition B</dd>
//!   </dl>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, TemplateChildNode};

static META: RuleMeta = RuleMeta {
    name: "html/no-duplicate-dt",
    description: "Disallow duplicate <dt> names in <dl>",
    category: RuleCategory::HtmlConformance,
    fixable: false,
    default_severity: Severity::Warning,
};

#[derive(Default)]
pub struct NoDuplicateDt;

impl Rule for NoDuplicateDt {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component || element.tag != "dl" {
            return;
        }

        let mut seen: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

        for child in &element.children {
            if let TemplateChildNode::Element(el) = child {
                if el.tag == "dt" {
                    let text = get_text_content(el);
                    let normalized = text.trim().to_string();
                    if normalized.is_empty() {
                        continue;
                    }

                    if let std::collections::hash_map::Entry::Vacant(entry) =
                        seen.entry(normalized.clone())
                    {
                        entry.insert(el.loc.start.offset);
                    } else {
                        let message = ctx.t_fmt(
                            "html/no-duplicate-dt.message",
                            &[("term", normalized.as_str())],
                        );
                        let help = ctx.t("html/no-duplicate-dt.help");
                        ctx.warn_with_help(message, &el.loc, help);
                    }
                }
            }
        }
    }
}

fn get_text_content(element: &ElementNode) -> String {
    let mut text = String::new();
    for child in &element.children {
        if let TemplateChildNode::Text(t) = child {
            text.push_str(t.content.as_str());
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::Linter;
    use crate::rule::RuleRegistry;

    fn create_linter() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(NoDuplicateDt));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_unique_dt() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<dl><dt>A</dt><dd>def A</dd><dt>B</dt><dd>def B</dd></dl>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_no_dt() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<dl></dl>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_not_dl() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div>text</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_duplicate_dt() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<dl><dt>A</dt><dd>def 1</dd><dt>A</dt><dd>def 2</dd></dl>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_triple_duplicate() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<dl><dt>X</dt><dd>1</dd><dt>X</dt><dd>2</dd><dt>X</dt><dd>3</dd></dl>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 2);
    }
}
