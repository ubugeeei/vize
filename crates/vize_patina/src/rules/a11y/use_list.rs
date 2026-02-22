//! a11y/use-list
//!
//! Detect text content that starts with bullet-like characters and suggest
//! using semantic list elements (`<ul>`, `<ol>`) instead.
//! Based on markuplint's `use-list` rule.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <p>- Item one</p>
//!   <p>- Item two</p>
//!   <p>* Another item</p>
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template>
//!   <ul>
//!     <li>Item one</li>
//!     <li>Item two</li>
//!   </ul>
//! </template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, TemplateChildNode};

static META: RuleMeta = RuleMeta {
    name: "a11y/use-list",
    description: "Suggest using list elements for bullet-like text",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Bullet-like prefixes that suggest list usage
const BULLET_CHARS: &[&str] = &[
    "- ",
    "* ",
    "+ ",
    "\u{2022} ", // •
    "\u{2023} ", // ‣
    "\u{25E6} ", // ◦
    "\u{2043} ", // ⁃
    "\u{2219} ", // ∙
    "\u{25AA} ", // ▪
    "\u{25CF} ", // ●
];

/// Elements that are already list-related
const LIST_CONTEXT_TAGS: &[&str] = &["ul", "ol", "li", "pre", "code", "script", "style"];

#[derive(Default)]
pub struct UseList;

impl Rule for UseList {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        let tag = element.tag.as_str();

        // Skip if already in list context
        if LIST_CONTEXT_TAGS.contains(&tag) {
            return;
        }

        // Skip if ancestor is list context
        if ctx.has_ancestor(|a| LIST_CONTEXT_TAGS.contains(&a.tag.as_str())) {
            return;
        }

        // Check first text child for bullet prefix
        for child in &element.children {
            if let TemplateChildNode::Text(text) = child {
                let trimmed = text.content.trim_start();
                if BULLET_CHARS
                    .iter()
                    .any(|prefix| trimmed.starts_with(prefix))
                {
                    let message = ctx.t("a11y/use-list.message");
                    let help = ctx.t("a11y/use-list.help");
                    ctx.warn_with_help(message, &text.loc, help);
                }
                // Only check the first meaningful text node
                if !text.content.trim().is_empty() {
                    break;
                }
            } else {
                break;
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
        registry.register(Box::new(UseList));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_normal_text() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<p>Normal text</p>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_list_element() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<ul><li>- Item with dash</li></ul>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_pre_element() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<pre>- markdown content</pre>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_code_element() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<code>- flag</code>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_dash_without_space() {
        let linter = create_linter();
        // "- " is a bullet, but "-word" is not
        let result = linter.lint_template(r#"<p>-word</p>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_dash_bullet() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<p>- Item one</p>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_asterisk_bullet() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<p>* Item one</p>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_plus_bullet() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<p>+ Item one</p>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_unicode_bullet() {
        let linter = create_linter();
        let result = linter.lint_template("<p>\u{2022} Item one</p>", "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_in_span() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<span>- item</span>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }
}
