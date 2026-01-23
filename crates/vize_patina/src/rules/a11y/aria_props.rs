//! a11y/aria-props
//!
//! Elements cannot use an invalid ARIA attribute.
//!
//! This rule fails if it finds an `aria-*` property that is not listed in the
//! WAI-ARIA States and Properties specification.
//!
//! Invalid ARIA attributes can prevent assistive technologies from conveying
//! the intended meaning to users.
//!
//! Based on eslint-plugin-jsx-a11y aria-props rule.

use crate::context::LintContext;
use crate::diagnostic::Severity;
use crate::rule::{Rule, RuleCategory, RuleMeta};
use vize_relief::ast::{ElementNode, PropNode, SourceLocation};

static META: RuleMeta = RuleMeta {
    name: "a11y/aria-props",
    description: "Disallow invalid ARIA attributes",
    category: RuleCategory::Accessibility,
    fixable: false,
    default_severity: Severity::Error,
};

/// Valid ARIA attributes from WAI-ARIA specification.
/// https://www.w3.org/TR/wai-aria/#state_prop_def
///
/// This list includes attributes from:
/// - WAI-ARIA 1.2 (https://www.w3.org/TR/wai-aria-1.2/)
/// - WAI-ARIA 1.3 draft additions
const VALID_ARIA_ATTRIBUTES: &[&str] = &[
    // === Global States and Properties ===
    // https://www.w3.org/TR/wai-aria/#global_states
    "aria-atomic",
    "aria-busy",
    "aria-controls",
    "aria-current",
    "aria-describedby",
    "aria-description", // ARIA 1.3
    "aria-details",
    "aria-disabled",
    "aria-dropeffect", // Deprecated in ARIA 1.1, but still valid
    "aria-errormessage",
    "aria-flowto",
    "aria-grabbed", // Deprecated in ARIA 1.1, but still valid
    "aria-haspopup",
    "aria-hidden",
    "aria-invalid",
    "aria-keyshortcuts",
    "aria-label",
    "aria-labelledby",
    "aria-live",
    "aria-owns",
    "aria-relevant",
    "aria-roledescription",
    // === Widget Attributes ===
    // https://www.w3.org/TR/wai-aria/#attrs_widgets
    "aria-autocomplete",
    "aria-checked",
    "aria-expanded",
    "aria-level",
    "aria-modal",
    "aria-multiline",
    "aria-multiselectable",
    "aria-orientation",
    "aria-placeholder",
    "aria-pressed",
    "aria-readonly",
    "aria-required",
    "aria-selected",
    "aria-sort",
    "aria-valuemax",
    "aria-valuemin",
    "aria-valuenow",
    "aria-valuetext",
    // === Relationship Attributes ===
    // https://www.w3.org/TR/wai-aria/#attrs_relationships
    "aria-activedescendant",
    "aria-colcount",
    "aria-colindex",
    "aria-colindextext", // ARIA 1.3
    "aria-colspan",
    "aria-posinset",
    "aria-rowcount",
    "aria-rowindex",
    "aria-rowindextext", // ARIA 1.3
    "aria-rowspan",
    "aria-setsize",
    // === ARIA 1.3 Braille Attributes ===
    // https://w3c.github.io/aria/#aria-braillelabel
    "aria-braillelabel",
    "aria-brailleroledescription",
];

/// Disallow invalid ARIA attributes
#[derive(Default)]
pub struct AriaProps;

impl AriaProps {
    /// Check if an attribute name is a valid ARIA attribute
    #[inline]
    fn is_valid_aria_attr(name: &str) -> bool {
        VALID_ARIA_ATTRIBUTES.contains(&name)
    }

    /// Check if an attribute name starts with "aria-"
    #[inline]
    fn is_aria_attr(name: &str) -> bool {
        name.starts_with("aria-")
    }

    /// Find similar valid ARIA attributes for suggestions
    fn find_similar(invalid: &str) -> Option<&'static str> {
        // Common typos mapping
        let typo_fixes: &[(&str, &str)] = &[
            ("aria-labeledby", "aria-labelledby"),
            ("aria-describeby", "aria-describedby"),
            ("aria-role", "role"),
            ("aria-labelled-by", "aria-labelledby"),
            ("aria-described-by", "aria-describedby"),
            ("aria-labelleby", "aria-labelledby"),
            ("aria-lable", "aria-label"),
            ("aria-lablledby", "aria-labelledby"),
            ("aria-hiiden", "aria-hidden"),
            ("aria-hdden", "aria-hidden"),
            ("aria-disbled", "aria-disabled"),
            ("aria-exanded", "aria-expanded"),
            ("aria-expandd", "aria-expanded"),
        ];

        // Check direct typo mapping first
        for (typo, fix) in typo_fixes {
            if *typo == invalid {
                return Some(fix);
            }
        }

        // Levenshtein-like similarity check for other cases
        let invalid_lower = invalid.to_ascii_lowercase();
        VALID_ARIA_ATTRIBUTES
            .iter()
            .find(|valid| {
                let valid_lower = valid.to_ascii_lowercase();
                // Simple similarity: same length +/- 2 and most chars match
                let len_diff = (invalid_lower.len() as i32 - valid_lower.len() as i32).abs();
                if len_diff > 2 {
                    return false;
                }
                // Count matching chars
                let matches = invalid_lower
                    .chars()
                    .zip(valid_lower.chars())
                    .filter(|(a, b)| a == b)
                    .count();
                matches >= invalid_lower.len().saturating_sub(2)
            })
            .copied()
    }
}

impl Rule for AriaProps {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn enter_element<'a>(&self, ctx: &mut LintContext<'a>, element: &ElementNode<'a>) {
        for prop in &element.props {
            match prop {
                PropNode::Attribute(attr) => {
                    let name = attr.name.as_str();
                    if Self::is_aria_attr(name) && !Self::is_valid_aria_attr(name) {
                        self.report_invalid_aria(ctx, name, &attr.loc);
                    }
                }
                PropNode::Directive(dir) => {
                    // Check v-bind:aria-* or :aria-*
                    if dir.name == "bind" {
                        if let Some(vize_relief::ast::ExpressionNode::Simple(arg)) = &dir.arg {
                            let name = arg.content.as_str();
                            if Self::is_aria_attr(name) && !Self::is_valid_aria_attr(name) {
                                self.report_invalid_aria(ctx, name, &dir.loc);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl AriaProps {
    fn report_invalid_aria(
        &self,
        ctx: &mut LintContext<'_>,
        invalid_attr: &str,
        loc: &SourceLocation,
    ) {
        let message = ctx.t_fmt("a11y/aria-props.message", &[("attr", invalid_attr)]);

        // Find suggestion for similar valid attribute
        if let Some(suggestion) = Self::find_similar(invalid_attr) {
            let help = ctx.t_fmt(
                "a11y/aria-props.help_suggestion",
                &[("invalid", invalid_attr), ("valid", suggestion)],
            );
            ctx.error_with_help(&message, loc, &help);
        } else {
            ctx.error_with_help(&message, loc, ctx.t("a11y/aria-props.help"));
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
        registry.register(Box::new(AriaProps));
        Linter::with_registry(registry)
    }

    #[test]
    fn test_valid_aria_labelledby() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input aria-labelledby="address" />"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_aria_label() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<button aria-label="Close">X</button>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_aria_hidden() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<div aria-hidden="true"></div>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_aria_expanded() {
        let linter = create_linter();
        let result =
            linter.lint_template(r#"<button aria-expanded="false">Menu</button>"#, "test.vue");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_multiple_aria_attrs() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<div role="dialog" aria-labelledby="title" aria-describedby="desc" aria-modal="true"></div>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_valid_dynamic_aria() {
        let linter = create_linter();
        let result = linter.lint_template(
            r#"<button :aria-expanded="isOpen">Toggle</button>"#,
            "test.vue",
        );
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_invalid_aria_labeledby() {
        let linter = create_linter();
        // Common typo: aria-labeledby (missing 'l')
        let result = linter.lint_template(r#"<input aria-labeledby="address" />"#, "test.vue");
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_invalid_aria_describeby() {
        let linter = create_linter();
        // Typo: aria-describeby (missing 'd')
        let result = linter.lint_template(r#"<div aria-describeby="desc"></div>"#, "test.vue");
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_invalid_made_up_aria() {
        let linter = create_linter();
        // Made up attribute
        let result = linter.lint_template(r#"<div aria-foobar="test"></div>"#, "test.vue");
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_invalid_dynamic_aria() {
        let linter = create_linter();
        let result = linter.lint_template(r#"<input :aria-labeledby="labelId" />"#, "test.vue");
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_find_similar_typos() {
        assert_eq!(
            AriaProps::find_similar("aria-labeledby"),
            Some("aria-labelledby")
        );
        assert_eq!(
            AriaProps::find_similar("aria-describeby"),
            Some("aria-describedby")
        );
    }

    #[test]
    fn test_is_valid_aria_attr() {
        assert!(AriaProps::is_valid_aria_attr("aria-label"));
        assert!(AriaProps::is_valid_aria_attr("aria-labelledby"));
        assert!(AriaProps::is_valid_aria_attr("aria-hidden"));
        assert!(!AriaProps::is_valid_aria_attr("aria-labeledby"));
        assert!(!AriaProps::is_valid_aria_attr("aria-foobar"));
    }
}
