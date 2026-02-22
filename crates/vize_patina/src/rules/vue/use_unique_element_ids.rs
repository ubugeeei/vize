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
//! ## Tiered Warnings
//!
//! When `tiered` is enabled (default), elements are categorized:
//! - **Form-related** (`input`, `select`, `textarea`, `button`, `label`, `fieldset`,
//!   `output`, `datalist`): Always warn — these rely on ID associations.
//! - **Landmark** (`h1`–`h6`, `section`, `nav`, `article`, `aside`, `main`,
//!   `header`, `footer`): Skip — these commonly use static IDs for hash navigation.
//! - **Other**: Only warn if the element also has ARIA reference attributes.
//!
//! ## Examples
//!
//! ### Invalid
//! ```vue
//! <template>
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

/// ARIA reference attributes (subset of ID_REFERENCE_ATTRIBUTES)
const ARIA_REFERENCE_ATTRIBUTES: &[&str] = &[
    "aria-labelledby",
    "aria-describedby",
    "aria-controls",
    "aria-owns",
    "aria-activedescendant",
    "aria-flowto",
    "aria-details",
    "aria-errormessage",
];

/// Element tier for ID warning classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdWarningTier {
    /// Form-related elements that rely on ID associations — always warn
    FormRelated,
    /// Landmark/heading elements — skip (commonly use static IDs for hash navigation)
    Landmark,
    /// Other elements — only warn if ARIA reference attributes are present
    Other,
}

/// Classify an element tag into a warning tier
fn classify_element(tag: &str) -> IdWarningTier {
    match tag {
        // Form-related elements
        "input" | "select" | "textarea" | "button" | "label" | "fieldset" | "output"
        | "datalist" => IdWarningTier::FormRelated,
        // Landmark / heading elements
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "section" | "nav" | "article" | "aside"
        | "main" | "header" | "footer" => IdWarningTier::Landmark,
        // Everything else
        _ => IdWarningTier::Other,
    }
}

/// Check if element has any ARIA reference attribute (static)
fn has_aria_reference_attr(element: &ElementNode) -> bool {
    element.props.iter().any(|prop| {
        if let PropNode::Attribute(attr) = prop {
            ARIA_REFERENCE_ATTRIBUTES.contains(&attr.name.as_str())
        } else {
            false
        }
    })
}

/// Enforce unique element IDs using useId()
pub struct UseUniqueElementIds {
    /// Whether to allow static IDs (opt-out)
    pub allow_static: bool,
    /// Whether to use tiered warnings based on element type (default: true)
    pub tiered: bool,
}

impl Default for UseUniqueElementIds {
    fn default() -> Self {
        Self {
            allow_static: false,
            tiered: true,
        }
    }
}

impl UseUniqueElementIds {
    /// Check if an attribute is an ID reference (pointing to another element's ID)
    #[inline]
    pub fn is_id_reference_attr(name: &str) -> bool {
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

        let tag = element.tag.as_str();
        let tier = classify_element(tag);

        for prop in &element.props {
            if let PropNode::Attribute(attr) = prop {
                let name = attr.name.as_str();

                // Check for static id attribute
                if name == "id" {
                    if let Some(value) = &attr.value {
                        if self.tiered {
                            match tier {
                                IdWarningTier::Landmark => {
                                    // Skip — landmark/heading IDs for hash navigation
                                    continue;
                                }
                                IdWarningTier::FormRelated => {
                                    self.report_static_id_tiered(
                                        ctx,
                                        &attr.loc,
                                        value.content.as_str(),
                                        false,
                                        "message_form",
                                    );
                                }
                                IdWarningTier::Other => {
                                    // Only warn if element has ARIA reference attributes
                                    if has_aria_reference_attr(element) {
                                        self.report_static_id_tiered(
                                            ctx,
                                            &attr.loc,
                                            value.content.as_str(),
                                            false,
                                            "message_aria_ref",
                                        );
                                    }
                                }
                            }
                        } else {
                            self.report_static_id(ctx, &attr.loc, value.content.as_str(), false);
                        }
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

    fn report_static_id_tiered(
        &self,
        ctx: &mut LintContext<'_>,
        loc: &SourceLocation,
        value: &str,
        is_reference: bool,
        message_key_suffix: &str,
    ) {
        let has_use_id = ctx
            .analysis()
            .map(|a| a.bindings.contains("useId"))
            .unwrap_or(false);

        let full_key = format!("vue/use-unique-element-ids.{message_key_suffix}");
        let message = if is_reference {
            ctx.t_fmt(
                "vue/use-unique-element-ids.message_reference",
                &[("value", value)],
            )
        } else {
            ctx.t_fmt(&full_key, &[("value", value)])
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

    fn create_linter_no_tiered() -> Linter {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(UseUniqueElementIds {
            allow_static: false,
            tiered: false,
        }));
        Linter::with_registry(registry)
    }

    // ===== Valid cases =====

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

    // ===== Tiered: Landmark elements are skipped =====

    #[test]
    fn test_tiered_skip_heading() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<h1 id="intro">Introduction</h1>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_tiered_skip_section() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<section id="about">About</section>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_tiered_skip_nav() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<nav id="main-nav">Nav</nav>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_tiered_skip_footer() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<footer id="site-footer">Footer</footer>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    // ===== Tiered: Form elements always warn =====

    #[test]
    fn test_tiered_warn_input() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input id="name" type="text" />"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_tiered_warn_select() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<select id="country"></select>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_tiered_warn_label() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<label id="name-label">Name</label>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    // ===== Tiered: Other elements only warn with ARIA reference =====

    #[test]
    fn test_tiered_skip_div_without_aria() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div id="panel">content</div>"#, "test.vue");
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_tiered_warn_div_with_aria_ref() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div id="panel" aria-labelledby="title">content</div>"#,
            "test.vue",
        );
        // id="panel" warns because ARIA ref is present, aria-labelledby="title" warns as reference
        assert_eq!(result.warning_count, 2);
    }

    // ===== Reference attributes always warn =====

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
        // input with aria-describedby: input is form-related so id check doesn't apply (no id attr),
        // but aria-describedby is a reference attribute → 1 warning
        assert_eq!(result.warning_count, 1);
    }

    // ===== tiered=false: classic behavior =====

    #[test]
    fn test_no_tiered_warns_heading() {
        let linter = create_linter_no_tiered();
        let result = linter.lint_template(r#"<h1 id="intro">Introduction</h1>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_no_tiered_warns_div_without_aria() {
        let linter = create_linter_no_tiered();
        let result = linter.lint_template(r#"<div id="panel">content</div>"#, "test.vue");
        assert_eq!(result.warning_count, 1);
    }

    #[test]
    fn test_no_tiered_invalid_multiple_static_ids() {
        let linter = create_linter_no_tiered();
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

    #[test]
    fn test_classify_element() {
        assert_eq!(classify_element("input"), IdWarningTier::FormRelated);
        assert_eq!(classify_element("select"), IdWarningTier::FormRelated);
        assert_eq!(classify_element("label"), IdWarningTier::FormRelated);
        assert_eq!(classify_element("h1"), IdWarningTier::Landmark);
        assert_eq!(classify_element("section"), IdWarningTier::Landmark);
        assert_eq!(classify_element("nav"), IdWarningTier::Landmark);
        assert_eq!(classify_element("div"), IdWarningTier::Other);
        assert_eq!(classify_element("span"), IdWarningTier::Other);
    }
}
