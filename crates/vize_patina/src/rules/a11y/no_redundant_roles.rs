//! a11y/no-redundant-roles
//!
//! Disallow redundant ARIA roles that match the element's implicit role.
//!
//! Some HTML elements have implicit ARIA roles. Adding a role attribute that
//! matches the implicit role is redundant and adds unnecessary noise.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <nav role="navigation">...</nav>
//! <button role="button">...</button>
//! ```
//!
//! ### Valid
//! ```vue
//! <nav>...</nav>
//! <div role="navigation">...</div>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType};

use super::helpers::{get_implicit_role, get_static_attribute_value};

static META: RuleMeta = RuleMeta {
    name: "a11y/no-redundant-roles",
    description: "Disallow redundant ARIA roles",
    category: RuleCategory::Accessibility,
    fixable: true,
    default_severity: Severity::Warning,
};

/// Disallow redundant ARIA roles
#[derive(Default)]
pub struct NoRedundantRoles;

impl Rule for NoRedundantRoles {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag_type == ElementType::Component {
            return;
        }

        let role_value = match get_static_attribute_value(element, "role") {
            Some(r) => r,
            None => return,
        };

        let implicit_role = get_implicit_role(&element.tag, element);

        if let Some(implicit) = implicit_role {
            if implicit == role_value {
                ctx.warn_with_help(
                    ctx.t_fmt(
                        "a11y/no-redundant-roles.message",
                        &[("tag", &element.tag), ("role", role_value)],
                    ),
                    &element.loc,
                    ctx.t("a11y/no-redundant-roles.help"),
                );
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
        registry.register(Box::new(NoRedundantRoles));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_no_role() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<nav>Navigation</nav>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_different_role() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div role="navigation">Navigation</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_nav_navigation() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<nav role="navigation">Navigation</nav>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_button_button() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<button role="button">Click</button>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_main_main() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<main role="main">Content</main>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_component_skipped() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<MyNav role="navigation">Navigation</MyNav>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }
}
