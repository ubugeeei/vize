//! vue/attribute-order
//!
//! Enforce a consistent order of attributes on elements.
//!
//! Following the Vue.js style guide recommendation, attributes should be
//! ordered as follows:
//!
//! 1. Definition: `is`
//! 2. List Rendering: `v-for`
//! 3. Conditionals: `v-if`, `v-else-if`, `v-else`, `v-show`, `v-cloak`
//! 4. Render Modifiers: `v-pre`, `v-once`
//! 5. Global Awareness: `id`
//! 6. Unique Attributes: `ref`, `key`
//! 7. Two-Way Binding: `v-model`
//! 8. Other Attributes: other bound/unbound attributes
//! 9. Events: `v-on`, `@`
//! 10. Content: `v-html`, `v-text`
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <div @click="onClick" v-if="show" id="main"></div>
//! ```
//!
//! ### Valid
//! ```vue
//! <div v-if="show" id="main" @click="onClick"></div>
//! ```

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, ExpressionNode, PropNode};

static META: RuleMeta = RuleMeta {
    name: "vue/attribute-order",
    description: "Enforce a consistent order of attributes",
    category: RuleCategory::Recommended,
    fixable: false, // Could be fixable in the future
    default_severity: Severity::Warning,
};

/// Attribute categories in order of priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AttrCategory {
    Definition,      // is
    ListRendering,   // v-for
    Conditionals,    // v-if, v-else-if, v-else, v-show, v-cloak
    RenderModifiers, // v-pre, v-once
    GlobalAwareness, // id
    UniqueAttrs,     // ref, key
    TwoWayBinding,   // v-model
    OtherDirectives, // v-custom, v-bind without special meaning
    OtherAttrs,      // class, :class, other attributes
    Events,          // v-on, @
    Content,         // v-html, v-text
}

impl AttrCategory {
    fn from_prop(prop: &PropNode) -> Self {
        match prop {
            PropNode::Attribute(attr) => {
                let name = attr.name.as_str();
                match name {
                    "is" => AttrCategory::Definition,
                    "id" => AttrCategory::GlobalAwareness,
                    "ref" | "key" => AttrCategory::UniqueAttrs,
                    _ => AttrCategory::OtherAttrs,
                }
            }
            PropNode::Directive(dir) => {
                let name = dir.name.as_str();
                let arg = dir.arg.as_ref().and_then(|a| match a {
                    ExpressionNode::Simple(s) => Some(s.content.as_str()),
                    _ => None,
                });

                match name {
                    "for" => AttrCategory::ListRendering,
                    "if" | "else-if" | "else" | "show" | "cloak" => AttrCategory::Conditionals,
                    "pre" | "once" => AttrCategory::RenderModifiers,
                    "model" => AttrCategory::TwoWayBinding,
                    "on" => AttrCategory::Events,
                    "html" | "text" => AttrCategory::Content,
                    "bind" => match arg {
                        Some("key") => AttrCategory::UniqueAttrs,
                        Some("is") => AttrCategory::Definition,
                        _ => AttrCategory::OtherAttrs,
                    },
                    "slot" => AttrCategory::OtherDirectives,
                    _ => AttrCategory::OtherDirectives,
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        match self {
            AttrCategory::Definition => "DEFINITION (is)",
            AttrCategory::ListRendering => "LIST_RENDERING (v-for)",
            AttrCategory::Conditionals => "CONDITIONALS (v-if/v-show)",
            AttrCategory::RenderModifiers => "RENDER_MODIFIERS (v-pre/v-once)",
            AttrCategory::GlobalAwareness => "GLOBAL (id)",
            AttrCategory::UniqueAttrs => "UNIQUE (ref/key)",
            AttrCategory::TwoWayBinding => "TWO_WAY_BINDING (v-model)",
            AttrCategory::OtherDirectives => "OTHER_DIRECTIVES",
            AttrCategory::OtherAttrs => "OTHER_ATTR",
            AttrCategory::Events => "EVENTS (v-on/@)",
            AttrCategory::Content => "CONTENT (v-html/v-text)",
        }
    }
}

/// Enforce attribute order
pub struct AttributeOrder;

impl Rule for AttributeOrder {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if element.props.len() < 2 {
            return;
        }

        let mut prev_category: Option<AttrCategory> = None;
        let mut prev_prop_name: Option<String> = None;

        for prop in element.props.iter() {
            let category = AttrCategory::from_prop(prop);
            let prop_name = match prop {
                PropNode::Attribute(attr) => attr.name.to_string(),
                PropNode::Directive(dir) => {
                    let arg = dir
                        .arg
                        .as_ref()
                        .map(|a| match a {
                            ExpressionNode::Simple(s) => format!(":{}", s.content),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    format!("v-{}{}", dir.name, arg)
                }
            };

            if let Some(prev_cat) = prev_category {
                if category < prev_cat {
                    let loc = match prop {
                        PropNode::Attribute(attr) => &attr.loc,
                        PropNode::Directive(dir) => &dir.loc,
                    };

                    ctx.warn_with_help(
                        format!(
                            "Attribute \"{}\" should be placed before \"{}\"",
                            prop_name,
                            prev_prop_name.as_deref().unwrap_or("previous attribute")
                        ),
                        loc,
                        format!(
                            "Expected order: {} comes before {}",
                            category.name(),
                            prev_cat.name()
                        ),
                    );
                }
            }

            prev_category = Some(category);
            prev_prop_name = Some(prop_name);
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
        registry.register(Box::new(AttributeOrder));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_order() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div v-if="show" id="main" ref="el" :class="cls" @click="onClick"></div>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_event_before_conditional() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<div @click="onClick" v-if="show"></div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_valid_v_for_before_v_if() {
        let linter = create_linter();
        // Note: v-for and v-if on same element is bad practice, but testing order
        let result = linter.lint_template(
            r#"<template v-for="item in items" :key="item.id"><div v-if="item.visible"></div></template>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_id_before_v_for() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div id="list" v-for="item in items" :key="item.id"></div>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }
}
