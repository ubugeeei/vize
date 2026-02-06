//! vue/prop-name-casing
//!
//! Enforce camelCase for prop names in templates.
//!
//! In Vue, props defined in camelCase in script should also be used
//! in camelCase in templates (not kebab-case or PascalCase).
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <MyComponent my-prop="value" />
//! ```
//!
//! ### Valid
//! ```vue
//! <MyComponent myProp="value" />
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ElementType, ExpressionNode, PropNode};

static META: RuleMeta = RuleMeta {
    name: "vue/prop-name-casing",
    description: "Enforce camelCase prop names in templates",
    category: RuleCategory::StronglyRecommended,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Enforce camelCase prop names
#[derive(Default)]
pub struct PropNameCasing;

fn is_kebab_case(s: &str) -> bool {
    s.contains('-')
}

fn kebab_to_camel(s: &str) -> String {
    let mut result = String::new();
    let mut upper_next = false;
    for c in s.chars() {
        if c == '-' {
            upper_next = true;
        } else if upper_next {
            result.push(c.to_ascii_uppercase());
            upper_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

impl Rule for PropNameCasing {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        // Only check props on components (native HTML elements use kebab-case attributes)
        if element.tag_type != ElementType::Component {
            return;
        }

        for prop in &element.props {
            match prop {
                PropNode::Attribute(attr) => {
                    let name = attr.name.as_str();
                    // Skip standard HTML attributes and directives
                    if name == "class"
                        || name == "style"
                        || name == "key"
                        || name == "ref"
                        || name == "is"
                    {
                        continue;
                    }
                    if is_kebab_case(name) {
                        let camel = kebab_to_camel(name);
                        ctx.warn_with_help(
                            ctx.t_fmt(
                                "vue/prop-name-casing.message",
                                &[("name", name), ("camel", &camel)],
                            ),
                            &attr.loc,
                            ctx.t("vue/prop-name-casing.help"),
                        );
                    }
                }
                PropNode::Directive(dir) => {
                    if dir.name == "bind" {
                        if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                            let name = arg.content.as_ref();
                            // Skip standard bindings
                            if name == "class"
                                || name == "style"
                                || name == "key"
                                || name == "ref"
                                || name == "is"
                            {
                                continue;
                            }
                            if is_kebab_case(name) {
                                let camel = kebab_to_camel(name);
                                ctx.warn_with_help(
                                    ctx.t_fmt(
                                        "vue/prop-name-casing.message",
                                        &[("name", name), ("camel", &camel)],
                                    ),
                                    &dir.loc,
                                    ctx.t("vue/prop-name-casing.help"),
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
        registry.register(Box::new(PropNameCasing));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_camel_case_prop() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<MyComponent myProp="value" />"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_native_element_kebab() {
        let linter = create_linter();
        // Native elements use kebab-case attributes - should not warn
        let result = linter.lint_template(r#"<div data-my-attr="value"></div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_kebab_case_prop() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<MyComponent my-prop="value" />"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_kebab_case_binding() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<MyComponent :my-prop="value" />"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_standard_attrs_skipped() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<MyComponent class="foo" :style="bar" key="1" />"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }
}
