//! a11y/anchor-is-valid
//!
//! Enforce that anchor elements have valid href attributes.
//!
//! Anchors with empty, `#`, or `javascript:void(0)` href values are
//! not valid and may cause accessibility issues.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <a href="">Link</a>
//! <a href="#">Link</a>
//! <a href="javascript:void(0)">Link</a>
//! ```
//!
//! ### Valid
//! ```vue
//! <a href="/about">About</a>
//! <a :href="url">Dynamic Link</a>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, ExpressionNode, PropNode};

static META: RuleMeta = RuleMeta {
    name: "a11y/anchor-is-valid",
    description: "Enforce valid href on anchor elements",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Enforce valid href on anchor elements
#[derive(Default)]
pub struct AnchorIsValid;

impl Rule for AnchorIsValid {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        if element.tag != "a" {
            return;
        }

        for prop in &element.props {
            match prop {
                PropNode::Attribute(attr) if attr.name == "href" => {
                    let value = attr
                        .value
                        .as_ref()
                        .map(|v| v.content.as_ref())
                        .unwrap_or("");
                    let trimmed = value.trim();

                    if trimmed.is_empty() {
                        ctx.warn_with_help(
                            ctx.t("a11y/anchor-is-valid.message_empty"),
                            &attr.loc,
                            ctx.t("a11y/anchor-is-valid.help"),
                        );
                    } else if trimmed == "#" {
                        ctx.warn_with_help(
                            ctx.t("a11y/anchor-is-valid.message_hash"),
                            &attr.loc,
                            ctx.t("a11y/anchor-is-valid.help"),
                        );
                    } else if trimmed.starts_with("javascript:") {
                        ctx.warn_with_help(
                            ctx.t("a11y/anchor-is-valid.message_javascript"),
                            &attr.loc,
                            ctx.t("a11y/anchor-is-valid.help"),
                        );
                    }
                    return;
                }
                PropNode::Directive(dir) if dir.name == "bind" => {
                    if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                        if arg.content == "href" {
                            // Dynamic binding - assume valid
                            return;
                        }
                    }
                }
                _ => {}
            }
        }

        // No href at all - this is covered by anchor-has-content,
        // but we also flag it here as it's a more specific issue
        ctx.warn_with_help(
            ctx.t("a11y/anchor-is-valid.message_missing"),
            &element.loc,
            ctx.t("a11y/anchor-is-valid.help"),
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
        registry.register(Box::new(AnchorIsValid));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_href() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<a href="/about">About</a>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_dynamic_href() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<a :href="url">Link</a>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_empty_href() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<a href="">Link</a>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_hash_href() {
        let linter = create_linter();
        let result = linter.lint_template(r##"<a href="#">Link</a>"##, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_javascript_href() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<a href="javascript:void(0)">Link</a>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_no_href() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<a>Link</a>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_component_skipped() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<NuxtLink>Link</NuxtLink>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }
}
