//! vue/a11y-img-alt
//!
//! Require alt attribute on <img> elements for accessibility.
//!
//! Images must have an alt attribute for screen readers and when images
//! fail to load. Decorative images should have an empty alt attribute.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <img src="/photo.jpg" />
//! <img :src="photo" />
//! ```
//!
//! ### Valid
//! ```vue
//! <!-- Informative image -->
//! <img src="/photo.jpg" alt="Team photo from company retreat" />
//!
//! <!-- Decorative image (empty alt) -->
//! <img src="/decoration.svg" alt="" />
//!
//! <!-- Dynamic alt -->
//! <img :src="photo" :alt="photoDescription" />
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::ElementNode;

static META: RuleMeta = RuleMeta {
    name: "vue/a11y-img-alt",
    description: "Require alt attribute on images for accessibility",
    category: RuleCategory::Recommended,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Require alt attribute on images
#[derive(Default)]
pub struct A11yImgAlt;

impl Rule for A11yImgAlt {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag != "img" {
            return;
        }

        // Check for alt attribute (static or dynamic)
        let has_alt = element.props.iter().any(|prop| match prop {
            vize_relief::ast::PropNode::Attribute(attr) => attr.name == "alt",
            vize_relief::ast::PropNode::Directive(dir) => {
                if dir.name == "bind" {
                    matches!(
                        &dir.arg,
                        Some(vize_relief::ast::ExpressionNode::Simple(s)) if s.content == "alt"
                    )
                } else {
                    false
                }
            }
        });

        if !has_alt {
            ctx.warn_with_help(
                "<img> elements must have an alt attribute for accessibility",
                &element.loc,
                "Add alt=\"description\" for informative images or alt=\"\" for decorative images",
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
        registry.register(Box::new(A11yImgAlt));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_with_alt() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<img src="/photo.jpg" alt="Photo" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_empty_alt() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<img src="/decoration.svg" alt="" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_with_dynamic_alt() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<img :src="photo" :alt="photoAlt" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_no_alt() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<img src="/photo.jpg" />"#, "test.vue");
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].message.contains("alt"));
    }
}
