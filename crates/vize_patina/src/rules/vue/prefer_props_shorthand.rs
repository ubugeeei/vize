//! vue/prefer-props-shorthand
//!
//! Recommend using shorthand syntax for props when the prop name matches the variable name.
//!
//! Vue 3.4+ supports shorthand syntax where `:foo="foo"` can be written as just `:foo`.
//! This makes the template more concise and easier to read.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <MyComponent :foo="foo" />
//! <MyComponent :user-name="userName" />
//! <MyComponent :count="count" :name="name" />
//! ```
//!
//! ### Valid
//! ```vue
//! <!-- Shorthand syntax (Vue 3.4+) -->
//! <MyComponent :foo />
//! <MyComponent :user-name />
//! <MyComponent :count :name />
//!
//! <!-- Different names are fine -->
//! <MyComponent :foo="bar" />
//! <MyComponent :count="totalCount" />
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ExpressionNode};

static META: RuleMeta = RuleMeta {
    name: "vue/prefer-props-shorthand",
    description: "Recommend shorthand syntax for props (Vue 3.4+)",
    category: RuleCategory::Recommended,
    fixable: true,
    default_severity: Severity::Warning,
};

/// Prefer props shorthand rule
#[derive(Default)]
pub struct PreferPropsShorthand;

impl PreferPropsShorthand {
    /// Convert kebab-case to camelCase
    fn kebab_to_camel(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut capitalize_next = false;

        for c in s.chars() {
            if c == '-' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Check if prop name and value name match (considering kebab-case conversion)
    fn names_match(prop_name: &str, value: &str) -> bool {
        // Direct match
        if prop_name == value {
            return true;
        }

        // Check if kebab-case prop matches camelCase value
        // e.g., user-name matches userName
        let camel_prop = Self::kebab_to_camel(prop_name);
        camel_prop == value
    }
}

impl Rule for PreferPropsShorthand {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        // Only check on component elements (PascalCase or kebab-case with -)
        let tag = element.tag.as_str();
        let is_component =
            tag.contains('-') || tag.chars().next().is_some_and(|c| c.is_uppercase());

        if !is_component {
            return;
        }

        for attr in &element.props {
            if let vize_relief::ast::PropNode::Directive(dir) = attr {
                if dir.name == "bind" {
                    if let Some(arg) = &dir.arg {
                        // Get the prop name
                        let prop_name = match arg {
                            ExpressionNode::Simple(s) => s.content.as_str(),
                            _ => continue,
                        };

                        // Get the expression value
                        if let Some(exp) = &dir.exp {
                            let value = match exp {
                                ExpressionNode::Simple(s) => s.content.trim(),
                                _ => continue,
                            };

                            // Check if it's a simple identifier matching the prop name
                            let is_simple_identifier =
                                value.chars().all(|c: char| c.is_alphanumeric() || c == '_');

                            if is_simple_identifier && Self::names_match(prop_name, value) {
                                ctx.warn_with_help(
                                    format!(
                                        "Use shorthand syntax: `:{}` instead of `:{}=\"{}\"`",
                                        prop_name, prop_name, value
                                    ),
                                    &dir.loc,
                                    format!("Use `:{}`", prop_name),
                                );
                            }
                        }
                    }
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
        registry.register(Box::new(PreferPropsShorthand));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_shorthand() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<MyComponent :foo />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_different_names() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<MyComponent :foo="bar" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_same_name() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<MyComponent :foo="foo" />"#, "test.vue");
        assert_eq!(result.warning_count, 1);
        assert!(result.diagnostics[0].message.contains("shorthand"));
    }

    #[test]
    fn test_invalid_kebab_camel_match() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<MyComponent :user-name="userName" />"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_expression() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<MyComponent :foo="foo + bar" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_kebab_to_camel() {
        assert_eq!(
            PreferPropsShorthand::kebab_to_camel("user-name"),
            "userName"
        );
        assert_eq!(
            PreferPropsShorthand::kebab_to_camel("foo-bar-baz"),
            "fooBarBaz"
        );
        assert_eq!(PreferPropsShorthand::kebab_to_camel("simple"), "simple");
    }
}
