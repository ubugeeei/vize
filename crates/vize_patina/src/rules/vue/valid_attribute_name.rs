//! vue/valid-attribute-name
//!
//! Require valid attribute names in templates.
//!
//! HTML attribute names must not contain spaces, quotes, equal signs,
//! or other invalid characters.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <div my"attr="value"></div>
//! <div 0abc="value"></div>
//! ```
//!
//! ### Valid
//! ```vue
//! <div my-attr="value"></div>
//! <div data-value="value"></div>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, PropNode};

static META: RuleMeta = RuleMeta {
    name: "vue/valid-attribute-name",
    description: "Require valid attribute names",
    category: RuleCategory::Essential,
    fixable: false,
    default_severity: Severity::Error,
};

#[derive(Default)]
pub struct ValidAttributeName;

impl ValidAttributeName {
    /// Check if an attribute name is valid according to HTML spec.
    /// Attribute names must not contain spaces, quotes, `>`, `/`, `=`,
    /// or noncharacters/control characters.
    fn is_valid_attribute_name(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        for ch in name.chars() {
            if ch.is_whitespace()
                || ch == '"'
                || ch == '\''
                || ch == '>'
                || ch == '/'
                || ch == '='
                || ch.is_control()
            {
                return false;
            }
        }

        true
    }
}

impl Rule for ValidAttributeName {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                if !Self::is_valid_attribute_name(&attr.name) {
                    ctx.error_with_help(
                        ctx.t_fmt("vue/valid-attribute-name.message", &[("name", &attr.name)]),
                        &attr.name_loc,
                        ctx.t("vue/valid-attribute-name.help"),
                    );
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
        registry.register(Box::new(ValidAttributeName));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_normal_attribute() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div class="foo"></div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_data_attribute() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div data-value="foo"></div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_aria_attribute() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div aria-label="foo"></div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }
}
