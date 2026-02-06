//! vue/no-useless-template-attributes
//!
//! Disallow useless attribute on `<template>`.
//!
//! `<template>` tags are not rendered to the DOM and only accept structural
//! directives (v-if, v-else-if, v-else, v-for, v-slot). All other attributes
//! and directives are silently ignored by Vue.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template v-if="show" class="foo"><div /></template>
//! <template v-for="item in items" id="bar"><div /></template>
//! <template v-slot:header ref="tmpl"><div /></template>
//! ```
//!
//! ### Valid
//! ```vue
//! <template v-if="show"><div /></template>
//! <template v-for="item in items" :key="item"><div /></template>
//! <template v-slot:header><div /></template>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, PropNode};

static META: RuleMeta = RuleMeta {
    name: "vue/no-useless-template-attributes",
    description: "Disallow useless attributes on `<template>` elements",
    category: RuleCategory::Essential,
    fixable: false,
    default_severity: Severity::Error,
};

#[derive(Default)]
pub struct NoUselessTemplateAttributes;

/// Structural directives that are valid on `<template>`
const STRUCTURAL_DIRECTIVES: &[&str] = &["if", "else-if", "else", "for", "slot"];

impl NoUselessTemplateAttributes {
    /// Check if a template has a structural directive (which justifies its existence)
    fn has_structural_directive(element: &ElementNode) -> bool {
        for prop in &element.props {
            if let PropNode::Directive(dir) = prop {
                if STRUCTURAL_DIRECTIVES.contains(&dir.name.as_str()) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a directive/attribute is allowed on `<template>`
    fn is_allowed_on_template(prop: &PropNode) -> bool {
        match prop {
            PropNode::Directive(dir) => {
                let name = dir.name.as_str();
                // Structural directives and key binding are allowed
                if STRUCTURAL_DIRECTIVES.contains(&name) {
                    return true;
                }
                // :key is allowed on v-for templates
                if name == "bind" {
                    if let Some(vize_relief::ast::ExpressionNode::Simple(arg)) = &dir.arg {
                        if arg.content == "key" {
                            return true;
                        }
                    }
                }
                false
            }
            PropNode::Attribute(attr) => {
                // key attribute is allowed
                attr.name == "key"
            }
        }
    }
}

impl Rule for NoUselessTemplateAttributes {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.tag.as_str() != "template" {
            return;
        }

        // Root template is the SFC wrapper, skip it
        if ctx.parent_element().is_none() {
            return;
        }

        // Only report if the template has a structural directive
        // (templates without structural directives are handled by no-lone-template)
        if !Self::has_structural_directive(element) {
            return;
        }

        for prop in &element.props {
            if !Self::is_allowed_on_template(prop) {
                let attr_name = match prop {
                    PropNode::Attribute(attr) => attr.name.to_string(),
                    PropNode::Directive(dir) => {
                        if let Some(raw) = &dir.raw_name {
                            raw.to_string()
                        } else {
                            format!("v-{}", dir.name)
                        }
                    }
                };
                ctx.error_with_help(
                    ctx.t_fmt(
                        "vue/no-useless-template-attributes.message",
                        &[("attr", &attr_name)],
                    ),
                    prop.loc(),
                    ctx.t("vue/no-useless-template-attributes.help"),
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
        registry.register(Box::new(NoUselessTemplateAttributes));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_v_if_only() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div><template v-if="show"><span>Hi</span></template></div>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_v_for_with_key() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div><template v-for="item in items" :key="item"><span /></template></div>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_v_slot() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<MyComponent><template v-slot:header><h1>Title</h1></template></MyComponent>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_class_on_template() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div><template v-if="show" class="foo"><span>Hi</span></template></div>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_invalid_id_on_template() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div><template v-for="item in items" id="bar"><span /></template></div>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 1);
    }
}
