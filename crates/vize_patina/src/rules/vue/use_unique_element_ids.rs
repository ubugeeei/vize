//! vue/use-unique-element-ids
//!
//! Enforce unique element IDs by using `useId()` instead of static string literals.
//!
//! Static `id` attributes can cause duplicate IDs when components are rendered
//! multiple times, leading to accessibility issues and broken functionality.
//!
//! The `useId()` composable (available in Vue 3.5+) generates unique IDs
//! that are stable across server and client rendering.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
//!   <div id="foo">bar</div>
//!   <label for="input">Name:</label>
//!   <input id="input" type="text" />
//! </template>
//! ```
//!
//! ### Valid
//! ```vue
//! <script setup>
//! import { useId } from 'vue'
//!
//! const id = useId()
//! </script>
//!
//! <template>
//!   <label :for="id">Name:</label>
//!   <input :id="id" type="text" />
//! </template>
//! ```
//!
//! Based on Biome's useUniqueElementIds rule.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, PropNode, SourceLocation};

static META: RuleMeta = RuleMeta {
    name: "vue/use-unique-element-ids",
    description: "Enforce unique element IDs using useId() instead of static literals",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Warning,
};

/// Attributes that take ID references (not the ID itself)
const ID_REFERENCE_ATTRIBUTES: &[&str] = &[
    "for",              // <label for="...">
    "aria-labelledby",  // ARIA reference
    "aria-describedby", // ARIA reference
    "aria-controls",    // ARIA reference
    "aria-owns",        // ARIA reference
    "aria-activedescendant",
    "aria-flowto",
    "aria-details",
    "aria-errormessage",
    "headers", // <td headers="...">
    "list",    // <input list="...">
    "form",    // <button form="...">
    "popovertarget",
    "anchor",
];

/// Enforce unique element IDs using useId()
#[derive(Default)]
pub struct UseUniqueElementIds {
    /// Whether to allow static IDs (opt-out)
    pub allow_static: bool,
}

impl UseUniqueElementIds {
    /// Check if an attribute is an ID reference (pointing to another element's ID)
    #[inline]
    fn is_id_reference_attr(name: &str) -> bool {
        ID_REFERENCE_ATTRIBUTES.contains(&name)
    }
}

impl Rule for UseUniqueElementIds {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        if self.allow_static {
            return;
        }

        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                let name = attr.name.as_str();

                // Check for static id attribute
                if name == "id" {
                    if let Some(value) = &attr.value {
                        self.report_static_id(ctx, &attr.loc, value.content.as_str(), false);
                    }
                }
                // Check for static ID reference attributes (for, aria-labelledby, etc.)
                else if Self::is_id_reference_attr(name) {
                    if let Some(value) = &attr.value {
                        self.report_static_id(ctx, &attr.loc, value.content.as_str(), true);
                    }
                }
            }
        }
    }
}

impl UseUniqueElementIds {
    fn report_static_id(
        &self,
        ctx: &mut LintContext<'_>,
        loc: &SourceLocation,
        value: &str,
        is_reference: bool,
    ) {
        // Check if useId is already imported/used
        let has_use_id = ctx
            .analysis()
            .map(|a| a.bindings.contains("useId"))
            .unwrap_or(false);

        let message = if is_reference {
            ctx.t_fmt(
                "vue/use-unique-element-ids.message_reference",
                &[("value", value)],
            )
        } else {
            ctx.t_fmt("vue/use-unique-element-ids.message", &[("value", value)])
        };

        let help = if has_use_id {
            ctx.t("vue/use-unique-element-ids.help_has_use_id")
        } else {
            ctx.t("vue/use-unique-element-ids.help")
        };

        ctx.warn_with_help(&message, loc, help);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::Linter;
    use crate::rule::RuleRegistry;

    fn create_linter() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(UseUniqueElementIds::default()));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_dynamic_id() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div :id="id">content</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_no_id() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div class="test">content</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_valid_dynamic_for() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<label :for="inputId">Name</label><input :id="inputId" />"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_invalid_static_id() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div id="foo">content</div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_static_for() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<label for="input">Name</label>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_static_aria_labelledby() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<span role="checkbox" aria-labelledby="tac"></span>"#,
            "test.vue",
        );
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_static_aria_describedby() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input aria-describedby="hint" />"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_invalid_multiple_static_ids() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div id="foo"><label for="bar">Label</label><input id="bar" /></div>"#,
            "test.vue",
        );
        // id="foo", for="bar", id="bar"
        assert_eq!(result.warning_count, 3);
    }

    #[test]
    fn test_is_id_reference_attr() {
        assert!(UseUniqueElementIds::is_id_reference_attr("for"));
        assert!(UseUniqueElementIds::is_id_reference_attr("aria-labelledby"));
        assert!(UseUniqueElementIds::is_id_reference_attr(
            "aria-describedby"
        ));
        assert!(!UseUniqueElementIds::is_id_reference_attr("id"));
        assert!(!UseUniqueElementIds::is_id_reference_attr("class"));
    }
}
